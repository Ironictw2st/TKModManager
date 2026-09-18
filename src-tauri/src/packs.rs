//! Discover mod packs: `data\*.pack` and `workshop\content\779340\<id>\*.pack`, typed by the
//! PFH header so vanilla packs are excluded and movie packs are recognised.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum PackType {
    Boot,
    Release,
    Patch,
    Mod,
    Movie,
    Unknown,
}

impl PackType {
    fn from_nibble(n: u32) -> Self {
        match n {
            0 => PackType::Boot,
            1 => PackType::Release,
            2 => PackType::Patch,
            3 => PackType::Mod,
            4 => PackType::Movie,
            _ => PackType::Unknown,
        }
    }
}

/// Read only the PFH header's type nibble. Old Workshop packs may carry an 8-byte `MFH` prefix
/// before the `PFHx` magic; skip it like rpfm does.
pub fn read_pack_type(path: &Path) -> Result<PackType, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut head = [0u8; 16];
    let n = f.read(&mut head).map_err(|e| e.to_string())?;
    if n < 8 {
        return Err("file too short".into());
    }
    pack_type_from_header(&head[..n])
}

/// Pure header decode (unit-tested): `PFHx` magic + u32 whose low nibble is the type.
pub fn pack_type_from_header(head: &[u8]) -> Result<PackType, String> {
    let start = if head.len() >= 12 && &head[0..3] == b"MFH" { 8 } else { 0 };
    let magic = head.get(start..start + 3).ok_or("short header")?;
    if magic != b"PFH" {
        let shown = String::from_utf8_lossy(&head[..4.min(head.len())]).into_owned();
        return Err(format!("not a pack (magic {shown:?})"));
    }
    let b = head.get(start + 4..start + 8).ok_or("short header")?;
    let bits = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    Ok(PackType::from_nibble(bits & 0xF))
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ModSource {
    Workshop,
    Data,
}

/// One user pack on disk. `key` is the stable identity used by profiles:
/// `ws:<workshop id>/<file>` or `data:<file>`.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub key: String,
    pub file: String,
    pub path: String,
    /// Folder the pack sits in (a Workshop item folder, or the game's data dir).
    pub dir: String,
    pub source: ModSource,
    pub workshop_id: Option<String>,
    pub pack_type: PackType,
    pub size: u64,
    /// Last-modified time, unix seconds.
    pub mtime: u64,
    pub preview_path: Option<String>,
}

pub fn make_key(source: ModSource, workshop_id: Option<&str>, file: &str) -> String {
    match source {
        ModSource::Workshop => format!("ws:{}/{}", workshop_id.unwrap_or(""), file),
        ModSource::Data => format!("data:{file}"),
    }
}

fn file_meta(path: &Path) -> (u64, u64) {
    match std::fs::metadata(path) {
        Ok(m) => {
            let mtime = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (m.len(), mtime)
        }
        Err(_) => (0, 0),
    }
}

fn is_pack(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pack"))
        .unwrap_or(false)
}

/// Scan both locations. Vanilla packs (boot/release/patch) are skipped; unreadable files are
/// logged and skipped. Order is unspecified (the UI sorts).
pub fn scan(data_dir: Option<&Path>, workshop_dir: Option<&Path>) -> Vec<ModEntry> {
    let mut out = Vec::new();
    if let Some(data) = data_dir {
        if let Ok(rd) = std::fs::read_dir(data) {
            for e in rd.flatten() {
                let p = e.path();
                if !p.is_file() || !is_pack(&p) {
                    continue;
                }
                if let Some(entry) = entry_for(&p, ModSource::Data, None) {
                    out.push(entry);
                }
            }
        }
    }
    if let Some(ws) = workshop_dir {
        if let Ok(rd) = std::fs::read_dir(ws) {
            for item in rd.flatten() {
                let dir = item.path();
                if !dir.is_dir() {
                    continue;
                }
                let id = item.file_name().to_string_lossy().into_owned();
                if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                let Ok(files) = std::fs::read_dir(&dir) else { continue };
                for f in files.flatten() {
                    let p = f.path();
                    if !p.is_file() || !is_pack(&p) {
                        continue;
                    }
                    if let Some(entry) = entry_for(&p, ModSource::Workshop, Some(&id)) {
                        out.push(entry);
                    }
                }
            }
        }
    }
    out
}

fn entry_for(path: &Path, source: ModSource, workshop_id: Option<&str>) -> Option<ModEntry> {
    let pack_type = match read_pack_type(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("packs: skipping {}: {e}", path.display());
            return None;
        }
    };
    if !matches!(pack_type, PackType::Mod | PackType::Movie) {
        return None;
    }
    let file = path.file_name()?.to_string_lossy().into_owned();
    let (size, mtime) = file_meta(path);
    let preview = {
        let png: PathBuf = path.with_extension("png");
        png.is_file().then(|| png.to_string_lossy().into_owned())
    };
    Some(ModEntry {
        key: make_key(source, workshop_id, &file),
        file,
        path: path.to_string_lossy().into_owned(),
        dir: path.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(),
        source,
        workshop_id: workshop_id.map(str::to_owned),
        pack_type,
        size,
        mtime,
        preview_path: preview,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_pfh5_types() {
        let mut h = b"PFH5".to_vec();
        h.extend_from_slice(&3u32.to_le_bytes());
        h.extend_from_slice(&[0; 8]);
        assert_eq!(pack_type_from_header(&h).unwrap(), PackType::Mod);
        let mut h = b"PFH5".to_vec();
        h.extend_from_slice(&(4u32 | 0x40).to_le_bytes());
        h.extend_from_slice(&[0; 8]);
        assert_eq!(pack_type_from_header(&h).unwrap(), PackType::Movie);
    }

    #[test]
    fn skips_mfh_prefix() {
        let mut h = b"MFH\0\0\0\0\0PFH5".to_vec();
        h.extend_from_slice(&1u32.to_le_bytes());
        assert_eq!(pack_type_from_header(&h).unwrap(), PackType::Release);
    }

    #[test]
    fn rejects_non_pack() {
        assert!(pack_type_from_header(b"PK\x03\x04\0\0\0\0\0\0\0\0").is_err());
    }

    #[test]
    fn keys() {
        assert_eq!(make_key(ModSource::Workshop, Some("42"), "a.pack"), "ws:42/a.pack");
        assert_eq!(make_key(ModSource::Data, None, "a.pack"), "data:a.pack");
    }
}
