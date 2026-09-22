//! Extract mod archives (.zip / .7z / .rar), keeping only the files a mod install needs.
//! The format is detected from the magic bytes, not the extension: Nexus file names are free text.

use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Zip,
    SevenZip,
    Rar,
}

pub fn detect(path: &Path) -> Result<Format, String> {
    let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut head = [0u8; 8];
    let n = f.read(&mut head).map_err(|e| format!("{}: {e}", path.display()))?;
    format_from_header(&head[..n]).ok_or_else(|| format!("{} is not a zip, 7z or rar archive", path.display()))
}

pub fn format_from_header(head: &[u8]) -> Option<Format> {
    if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        Some(Format::Zip)
    } else if head.starts_with(b"7z\xBC\xAF\x27\x1C") {
        Some(Format::SevenZip)
    } else if head.starts_with(b"Rar!\x1A\x07") {
        Some(Format::Rar)
    } else {
        None
    }
}

/// Join an archive entry name onto `dest`, refusing anything that would leave it.
fn safe_join(dest: &Path, name: &str) -> Option<PathBuf> {
    let mut out = dest.to_path_buf();
    let mut any = false;
    for c in Path::new(&name.replace('\\', "/")).components() {
        match c {
            Component::Normal(p) => {
                out.push(p);
                any = true;
            }
            Component::CurDir => {}
            _ => return None,
        }
    }
    any.then_some(out)
}

/// Extract the entries `keep` accepts (called with the entry's file name, lower-cased) into
/// `dest`, keeping their folders. `dest` is created fresh. Returns the extracted paths.
pub fn extract_filtered(archive: &Path, dest: &Path, keep: &dyn Fn(&str) -> bool) -> Result<Vec<PathBuf>, String> {
    let _ = std::fs::remove_dir_all(dest);
    std::fs::create_dir_all(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let wanted = |name: &str| {
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name).to_ascii_lowercase();
        !base.is_empty() && keep(&base)
    };
    let mut out = Vec::new();
    match detect(archive)? {
        Format::Zip => {
            let file = std::fs::File::open(archive).map_err(|e| format!("{}: {e}", archive.display()))?;
            let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("bad zip: {e}"))?;
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i).map_err(|e| format!("bad zip: {e}"))?;
                if entry.is_dir() || !wanted(entry.name()) {
                    continue;
                }
                let target = safe_join(dest, entry.name()).ok_or_else(|| format!("unsafe path in archive: {}", entry.name()))?;
                write_entry(&target, &mut entry)?;
                out.push(target);
            }
        }
        Format::SevenZip => {
            sevenz_rust2::decompress_file_with_extract_fn(archive, dest, |entry, reader, target| {
                if entry.is_directory() || !wanted(entry.name()) {
                    // The reader must still be drained for solid archives.
                    std::io::copy(reader, &mut std::io::sink())?;
                    return Ok(true);
                }
                write_entry(target, reader).map_err(std::io::Error::other)?;
                out.push(target.clone());
                Ok(true)
            })
            .map_err(|e| format!("bad 7z: {e}"))?;
        }
        Format::Rar => {
            let mut rar = unrar::Archive::new(archive).open_for_processing().map_err(|e| format!("bad rar: {e}"))?;
            while let Some(header) = rar.read_header().map_err(|e| format!("bad rar: {e}"))? {
                let name = header.entry().filename.to_string_lossy().into_owned();
                rar = if header.entry().is_file() && wanted(&name) {
                    let target = safe_join(dest, &name).ok_or_else(|| format!("unsafe path in archive: {name}"))?;
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
                    }
                    let next = header.extract_to(&target).map_err(|e| format!("rar: {name}: {e}"))?;
                    out.push(target);
                    next
                } else {
                    header.skip().map_err(|e| format!("bad rar: {e}"))?
                };
            }
        }
    }
    Ok(out)
}

fn write_entry(target: &Path, reader: &mut dyn Read) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let mut f = std::fs::File::create(target).map_err(|e| format!("{}: {e}", target.display()))?;
    std::io::copy(reader, &mut f).map_err(|e| format!("{}: {e}", target.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn detects_formats() {
        assert_eq!(format_from_header(b"PK\x03\x04rest"), Some(Format::Zip));
        assert_eq!(format_from_header(b"7z\xBC\xAF\x27\x1C\0\x04"), Some(Format::SevenZip));
        assert_eq!(format_from_header(b"Rar!\x1A\x07\x01\0"), Some(Format::Rar));
        assert_eq!(format_from_header(b"PFH5"), None);
    }

    #[test]
    fn safe_join_rejects_escapes() {
        let d = Path::new("/x");
        assert!(safe_join(d, "../evil.pack").is_none());
        assert!(safe_join(d, "a/../../evil.pack").is_none());
        assert!(safe_join(d, "/abs.pack").is_none());
        assert_eq!(safe_join(d, r"sub\a.pack").unwrap(), Path::new("/x/sub/a.pack"));
    }

    #[test]
    fn zip_keeps_only_wanted_files() {
        let tmp = tempfile::tempdir().unwrap();
        let zp = tmp.path().join("m.zip");
        {
            let mut w = zip::ZipWriter::new(std::fs::File::create(&zp).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            w.start_file("Mod/a.pack", o).unwrap();
            w.write_all(b"PFH5").unwrap();
            w.start_file("readme.txt", o).unwrap();
            w.write_all(b"hi").unwrap();
            w.finish().unwrap();
        }
        let out = extract_filtered(&zp, &tmp.path().join("x"), &|n| n.ends_with(".pack")).unwrap();
        assert_eq!(out, vec![tmp.path().join("x").join("Mod").join("a.pack")]);
        assert!(!tmp.path().join("x").join("readme.txt").exists());
    }
}
