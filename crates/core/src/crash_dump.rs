//! Crash dumps for the "Send report" feature: a compact text summary read on the player's PC
//! (so triage works even when the dump itself is too big for Discord) and the packaging of the
//! dump itself (zip or 7z, split into parts that fit one Discord message each).
//!
//! The summary is plain English: the report is read by the developers, not shown in the UI.

use crate::paths;
use minidump::{Minidump, MinidumpException, MinidumpMiscInfo, MinidumpModuleList, MinidumpSystemInfo, MinidumpThreadList, Module};
use std::fmt::Write as _;
use std::path::Path;

/// How many frames / stack hits the summary lists.
const MAX_FRAMES: usize = 20;
/// How many bytes of stack above the stack pointer are scanned for return addresses.
const STACK_SCAN_BYTES: usize = 64 * 1024;

/// An address as `module+offset` (or the bare address when it is inside no module).
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub address: u64,
    /// File name of the module the address is inside.
    pub module: Option<String>,
    pub offset: u64,
}

impl Frame {
    pub fn text(&self) -> String {
        match &self.module {
            Some(m) => format!("{m}+0x{:X}", self.offset),
            None => format!("0x{:X}", self.address),
        }
    }
}

/// Everything the summary is rendered from (gathered by `read_summary`, kept separate for tests).
#[derive(Debug, Clone, Default)]
pub struct CrashInfo {
    pub os: String,
    pub exception_code: u32,
    pub exception_name: String,
    /// "read" / "write" / "execute" and the address, for access violations.
    pub access: Option<(String, u64)>,
    pub thread_id: u32,
    /// Where the faulting instruction is.
    pub fault: Option<Frame>,
    /// Return addresses found by scanning the faulting thread's stack (nearest first).
    pub stack_hits: Vec<Frame>,
    /// (file name, version or empty) of the loaded modules that matter: the game exe and the
    /// script extender; every other module is only counted.
    pub modules: Vec<(String, String)>,
    pub module_count: usize,
}

/// File name of a Windows or POSIX path.
fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// `1.2.3.4` from the high and low halves of a Windows file version.
fn version_text(hi: u32, lo: u32) -> String {
    if hi == 0 && lo == 0 {
        return String::new();
    }
    format!("{}.{}.{}.{}", hi >> 16, hi & 0xFFFF, lo >> 16, lo & 0xFFFF)
}

fn frame_for(modules: &MinidumpModuleList, address: u64) -> Frame {
    match modules.module_at_address(address) {
        Some(m) => Frame { address, module: Some(file_name(&m.code_file()).to_string()), offset: address - m.base_address() },
        None => Frame { address, module: None, offset: 0 },
    }
}

/// Modules worth naming in the summary: the game exe and the script extender.
fn is_interesting(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == paths::EXE_NAME.to_ascii_lowercase() || n == crate::dll::DLL_NAME || n.contains("script_extender")
}

/// Pointer-sized values on the stack that point into a module, nearest the stack pointer first.
/// There is no unwind info in these dumps, so this is a guess at the call chain: every return
/// address is in there, and so is some noise.
pub fn scan_stack(stack: &[u8], base: u64, sp: u64, modules: &MinidumpModuleList, max: usize) -> Vec<Frame> {
    let start = sp.saturating_sub(base) as usize;
    let start = (start + 7) & !7;
    let mut out: Vec<Frame> = Vec::new();
    let end = stack.len().min(start.saturating_add(STACK_SCAN_BYTES));
    let mut i = start;
    while i + 8 <= end && out.len() < max {
        let v = u64::from_le_bytes(stack[i..i + 8].try_into().unwrap());
        if modules.module_at_address(v).is_some() {
            let f = frame_for(modules, v);
            if out.last() != Some(&f) {
                out.push(f);
            }
        }
        i += 8;
    }
    out
}

/// Read the exception, the faulting thread's stack and the module list out of a minidump.
pub fn read_summary(path: &Path) -> Result<CrashInfo, String> {
    let dump = Minidump::read_path(path).map_err(|e| format!("cannot parse the dump: {e}"))?;
    let system: MinidumpSystemInfo = dump.get_stream().map_err(|e| format!("no system info in the dump: {e}"))?;
    let modules: MinidumpModuleList = dump.get_stream().map_err(|e| format!("no module list in the dump: {e}"))?;
    let misc = dump.get_stream::<MinidumpMiscInfo>().ok();
    let mut info = CrashInfo { os: format!("{} {}", system.os, system.cpu), module_count: modules.iter().count(), ..Default::default() };

    for m in modules.iter() {
        let name = file_name(&m.code_file()).to_string();
        if is_interesting(&name) {
            let v = &m.raw.version_info;
            info.modules.push((name, version_text(v.file_version_hi, v.file_version_lo)));
        }
    }

    let exc: MinidumpException = dump.get_stream().map_err(|e| format!("no exception in the dump: {e}"))?;
    info.thread_id = exc.get_crashing_thread_id();
    info.exception_code = exc.raw.exception_record.exception_code;
    info.exception_name = format!("{}", exc.get_crash_reason(system.os, system.cpu));
    let rec = &exc.raw.exception_record;
    if info.exception_code == 0xC000_0005 && rec.number_parameters >= 2 {
        let kind = match rec.exception_information[0] {
            0 => "read",
            1 => "write",
            8 => "execute",
            _ => "access",
        };
        info.access = Some((kind.to_string(), rec.exception_information[1]));
    }

    let ctx = exc.context(&system, misc.as_ref());
    let rip = ctx.as_ref().map(|c| c.get_instruction_pointer()).unwrap_or(rec.exception_address);
    info.fault = Some(frame_for(&modules, rip));

    if let (Some(ctx), Ok(threads), Some(mem)) = (ctx.as_ref(), dump.get_stream::<MinidumpThreadList>(), dump.get_memory()) {
        if let Some(t) = threads.threads.iter().find(|t| t.raw.thread_id == info.thread_id) {
            let sp = ctx.get_stack_pointer();
            let region = mem.memory_at_address(sp).or_else(|| t.stack_memory(&mem));
            if let Some(r) = region {
                info.stack_hits = scan_stack(r.bytes(), r.base_address(), sp, &modules, MAX_FRAMES);
            }
        }
    }
    Ok(info)
}

/// A one-line crash signature: exception, where, e.g. `0xC0000005 Three_Kingdoms.exe+0x1A2B3C`.
pub fn signature(info: &CrashInfo) -> String {
    let at = info.fault.as_ref().map(|f| f.text()).unwrap_or_else(|| "unknown address".into());
    format!("0x{:08X} {} at {}", info.exception_code, info.exception_name, at)
}

/// The text block that goes into the report.
pub fn format_summary(info: &CrashInfo) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Crash signature: {}", signature(info));
    let _ = writeln!(out, "Exception: 0x{:08X} {}", info.exception_code, info.exception_name);
    if let Some((kind, addr)) = &info.access {
        let _ = writeln!(out, "Access violation: tried to {kind} 0x{addr:X}");
    }
    if let Some(f) = &info.fault {
        let _ = writeln!(out, "Faulting instruction: {} (thread {})", f.text(), info.thread_id);
    }
    if info.stack_hits.is_empty() {
        let _ = writeln!(out, "Stack: no readable stack for the faulting thread");
    } else {
        let _ = writeln!(out, "Stack, return addresses found on the faulting thread's stack (no unwind info, so some are noise), nearest first:");
        for (i, f) in info.stack_hits.iter().enumerate() {
            let _ = writeln!(out, "  #{i:<2} {}", f.text());
        }
    }
    for (name, v) in &info.modules {
        let _ = writeln!(out, "Module: {name} {}", if v.is_empty() { "(no version)" } else { v });
    }
    let _ = writeln!(out, "{} modules loaded, {}", info.module_count, info.os);
    out
}

/// Summary of one dump as report text; an explanation instead when the dump cannot be read.
pub fn summary_text(path: &Path) -> String {
    match read_summary(path) {
        Ok(i) => format_summary(&i),
        Err(e) => format!("Crash summary unavailable: {e}\n"),
    }
}

// ---------------------------------------------------------------- packaging

/// Smallest the first message's attachment budget may be: the report text shares the message.
pub const MIN_FIRST_PART: usize = 1024 * 1024;

/// A packed dump: the archive bytes, the file name to give it and how it was made.
pub struct Packed {
    pub name: &'static str,
    pub method: &'static str,
    pub bytes: Vec<u8>,
}

fn pack_entries(dump: &Path) -> Vec<(String, Vec<u8>)> {
    [dump.to_path_buf(), dump.with_extension("stack.txt")]
        .into_iter()
        .filter_map(|p| {
            let bytes = std::fs::read(&p).ok()?;
            Some((p.file_name()?.to_string_lossy().into_owned(), bytes))
        })
        .collect()
}

/// The dump and its stack file as a zip at the strongest deflate level (a 47 MB dump: ~10 MB).
pub fn zip_entries(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    use std::io::Write as _;
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_level(Some(9));
    for (name, bytes) in entries {
        w.start_file(name.as_str(), opts).map_err(|e| e.to_string())?;
        w.write_all(bytes).map_err(|e| e.to_string())?;
    }
    Ok(w.finish().map_err(|e| e.to_string())?.into_inner())
}

/// The same files as a 7z archive (LZMA2): about half the size of the zip for these dumps.
pub fn sevenz_entries(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut w = sevenz_rust2::ArchiveWriter::new(std::io::Cursor::new(Vec::new())).map_err(|e| e.to_string())?;
    for (name, bytes) in entries {
        w.push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(std::io::Cursor::new(bytes.as_slice()))).map_err(|e| e.to_string())?;
    }
    Ok(w.finish().map_err(|e| e.to_string())?.into_inner())
}

/// Pack the dump: deflate zip when it fits in `fits` bytes (opens everywhere), otherwise
/// whichever of zip and 7z is smaller.
pub fn pack(dump: &Path, fits: usize) -> Result<Packed, String> {
    let entries = pack_entries(dump);
    if entries.is_empty() {
        return Err("cannot read the dump".into());
    }
    let zip = zip_entries(&entries)?;
    if zip.len() <= fits {
        return Ok(Packed { name: "crash_dump.zip", method: "zip", bytes: zip });
    }
    match sevenz_entries(&entries) {
        Ok(z7) if z7.len() < zip.len() => Ok(Packed { name: "crash_dump.7z", method: "7z", bytes: z7 }),
        _ => Ok(Packed { name: "crash_dump.zip", method: "zip", bytes: zip }),
    }
}

/// Byte ranges of the parts an archive of `total` bytes is posted in: the first holds at most
/// `first_cap` (it shares its message with the report), the others at most `cap`. None when
/// that takes more than `max_parts`, or when a cap is zero.
pub fn split_ranges(total: usize, first_cap: usize, cap: usize, max_parts: usize) -> Option<Vec<std::ops::Range<usize>>> {
    if first_cap == 0 || cap == 0 || total == 0 {
        return None;
    }
    let mut out = Vec::new();
    let mut at = 0;
    while at < total {
        let size = if out.is_empty() { first_cap } else { cap };
        let end = (at + size).min(total);
        out.push(at..end);
        at = end;
        if out.len() > max_parts {
            return None;
        }
    }
    Some(out)
}

/// `crash_dump.zip.001`, `.002`, ... (a single part keeps the plain name).
pub fn part_name(base: &str, index: usize, parts: usize) -> String {
    if parts <= 1 {
        base.to_string()
    } else {
        format!("{base}.{:03}", index + 1)
    }
}

/// The command that joins downloaded parts back into the archive.
pub fn join_command(base: &str, parts: usize) -> String {
    let names: Vec<String> = (0..parts).map(|i| part_name(base, i, parts)).collect();
    format!("copy /b {} {base}", names.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> CrashInfo {
        CrashInfo {
            os: "windows amd64".into(),
            exception_code: 0xC000_0005,
            exception_name: "EXCEPTION_ACCESS_VIOLATION_READ".into(),
            access: Some(("read".into(), 0x18)),
            thread_id: 4242,
            fault: Some(Frame { address: 0x1_4012_3456, module: Some("Three_Kingdoms.exe".into()), offset: 0x123456 }),
            stack_hits: vec![
                Frame { address: 0x7FF8_0000_1000, module: Some("script_extender.dll".into()), offset: 0x1000 },
                Frame { address: 0x42, module: None, offset: 0 },
            ],
            modules: vec![("Three_Kingdoms.exe".into(), "1.6.0.12".into()), ("script_extender.dll".into(), String::new())],
            module_count: 310,
        }
    }

    #[test]
    fn summary_names_exception_modules_and_versions() {
        let t = format_summary(&info());
        assert!(t.starts_with("Crash signature: 0xC0000005 EXCEPTION_ACCESS_VIOLATION_READ at Three_Kingdoms.exe+0x123456\n"));
        assert!(t.contains("tried to read 0x18"));
        assert!(t.contains("#0  script_extender.dll+0x1000"));
        assert!(t.contains("#1  0x42"));
        assert!(t.contains("Module: Three_Kingdoms.exe 1.6.0.12"));
        assert!(t.contains("Module: script_extender.dll (no version)"));
        assert!(t.contains("310 modules loaded"));
        // One line of signature is what the Discord post carries.
        assert!(signature(&info()).chars().count() < 120);
    }

    #[test]
    fn versions_and_names() {
        assert_eq!(version_text(0x0001_0006, 0x0000_000C), "1.6.0.12");
        assert_eq!(version_text(0, 0), "");
        assert_eq!(file_name(r"C:\Games\Three_Kingdoms.exe"), "Three_Kingdoms.exe");
        assert_eq!(file_name("a.dll"), "a.dll");
    }

    #[test]
    fn split_sizing() {
        let mb = 1024 * 1024;
        // Fits as one part.
        assert_eq!(split_ranges(5 * mb, 9 * mb, 9 * mb, 6), Some(vec![0..5 * mb]));
        // 13 MB: first part is smaller because the report shares its message.
        let r = split_ranges(13 * mb, 9 * mb, 9 * mb, 6).unwrap();
        assert_eq!(r, vec![0..9 * mb, 9 * mb..13 * mb]);
        // Parts are contiguous and cover everything.
        let r = split_ranges(50 * mb + 7, 9 * mb, 9 * mb, 6).unwrap();
        assert_eq!(r.len(), 6);
        assert_eq!(r.first().unwrap().start, 0);
        assert_eq!(r.last().unwrap().end, 50 * mb + 7);
        assert!(r.windows(2).all(|w| w[0].end == w[1].start));
        assert!(r.iter().skip(1).all(|p| p.len() <= 9 * mb));
        // Exactly the cap on the last part still counts; one byte more is a seventh part.
        assert!(split_ranges(6 * 9 * mb, 9 * mb, 9 * mb, 6).is_some());
        assert!(split_ranges(6 * 9 * mb + 1, 9 * mb, 9 * mb, 6).is_none());
        assert!(split_ranges(10, 0, 5, 6).is_none());
    }

    #[test]
    fn part_names_and_join() {
        assert_eq!(part_name("crash_dump.zip", 0, 1), "crash_dump.zip");
        assert_eq!(part_name("crash_dump.7z", 1, 3), "crash_dump.7z.002");
        assert_eq!(join_command("crash_dump.zip", 2), "copy /b crash_dump.zip.001+crash_dump.zip.002 crash_dump.zip");
    }

    #[test]
    fn archives_round_trip() {
        let data: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let entries = vec![("D1.mdmp".to_string(), data.clone()), ("D1.stack.txt".to_string(), b"Three_Kingdoms\n".to_vec())];
        let z = zip_entries(&entries).unwrap();
        let mut a = zip::ZipArchive::new(std::io::Cursor::new(z)).unwrap();
        let mut got = Vec::new();
        std::io::Read::read_to_end(&mut a.by_name("D1.mdmp").unwrap(), &mut got).unwrap();
        assert_eq!(got, data);

        let s = sevenz_entries(&entries).unwrap();
        assert_eq!(&s[..6], b"7z\xBC\xAF\x27\x1C");
        let dir = tempfile::tempdir().unwrap();
        sevenz_rust2::decompress(std::io::Cursor::new(s), dir.path()).unwrap();
        assert_eq!(std::fs::read(dir.path().join("D1.mdmp")).unwrap(), data);
    }

    #[test]
    fn unreadable_dump_gives_a_note_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bad.mdmp");
        std::fs::write(&p, b"not a minidump").unwrap();
        assert!(summary_text(&p).starts_with("Crash summary unavailable"));
    }

    /// Needs a real dump on this PC (they are too big and third-party to commit).
    #[test]
    fn real_dump_summary() {
        let Ok(p) = std::env::var("TKMM_TEST_DUMP") else { return };
        let t = summary_text(Path::new(&p));
        println!("{t}");
        assert!(t.contains("Crash signature:"), "{t}");
    }

    #[test]
    fn real_dump_sizes() {
        let Ok(p) = std::env::var("TKMM_TEST_DUMP") else { return };
        let entries = pack_entries(Path::new(&p));
        let t = std::time::Instant::now();
        let z = zip_entries(&entries).unwrap();
        println!("zip deflate9: {} bytes ({:?})", z.len(), t.elapsed());
        let t = std::time::Instant::now();
        let s = sevenz_entries(&entries).unwrap();
        println!("7z lzma2:     {} bytes ({:?})", s.len(), t.elapsed());
        std::fs::write(std::env::temp_dir().join("tkmm_test_dump.7z"), &s).unwrap();
        assert!(s.len() < z.len());
    }
}
