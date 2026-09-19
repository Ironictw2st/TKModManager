//! Profile export / import text and the co-op sync check.
//!
//! Export v2 (tab separated): `key<TAB>file<TAB>size<TAB>sha256` per enabled pack, in load order.
//! v1 lines (`key` or `key<TAB># file`) are still accepted by the parser.

use crate::hash::PackHash;
use crate::packs::{ModEntry, ModSource};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub struct ExportRow {
    pub key: String,
    pub file: String,
    pub size: Option<u64>,
    pub sha256: Option<String>,
}

pub fn build_export(profile_name: &str, rows: &[PackHash], date: &str) -> String {
    let mut out = vec![format!("# TK Mod Manager profile \"{profile_name}\" {date} v2")];
    out.extend(rows.iter().map(|r| format!("{}\t{}\t{}\t{}", r.key, r.file, r.size, r.sha256)));
    out.join("\n")
}

pub fn parse_export(text: &str) -> Vec<ExportRow> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = raw.split('\t').map(str::trim).collect();
        let key = cols[0];
        if !(key.starts_with("ws:") || key.starts_with("data:") || key.starts_with("ext:")) {
            continue;
        }
        let second = cols.get(1).copied().unwrap_or("");
        let file = if let Some(f) = second.strip_prefix('#') {
            f.trim().to_string()
        } else if !second.is_empty() {
            second.to_string()
        } else {
            let tail = key.rsplit('/').next().unwrap_or(key);
            tail.trim_start_matches("data:").trim_start_matches("ext:").to_string()
        };
        let size = cols.get(2).and_then(|s| s.parse::<u64>().ok());
        let sha256 = cols.get(3).filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())).map(|s| s.to_lowercase());
        out.push(ExportRow { key: key.to_string(), file, size, sha256 });
    }
    out
}

/// The local pack an exported row refers to: Workshop rows by id + file, the rest by file name
/// (a partner's data/ or folder path differs from ours).
pub fn match_local<'a>(row: &ExportRow, mods: &'a [ModEntry]) -> Option<&'a ModEntry> {
    if row.key.starts_with("ws:") {
        return mods.iter().find(|m| m.key == row.key);
    }
    let file = row.file.to_lowercase();
    mods.iter()
        .find(|m| m.key == row.key)
        .or_else(|| mods.iter().find(|m| m.source != ModSource::Workshop && m.file.to_lowercase() == file))
        .or_else(|| mods.iter().find(|m| m.file.to_lowercase() == file))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncStatus {
    Match,
    Missing,
    Disabled,
    Different,
    Unverified,
}

impl SyncStatus {
    pub fn label(self) -> &'static str {
        match self {
            SyncStatus::Match => "identical",
            SyncStatus::Missing => "not installed",
            SyncStatus::Disabled => "installed, not enabled",
            SyncStatus::Different => "different file",
            SyncStatus::Unverified => "present (no hash)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SyncRow {
    pub row: ExportRow,
    pub local: Option<ModEntry>,
    pub status: SyncStatus,
}

#[derive(Clone, Debug)]
pub struct SyncResult {
    pub rows: Vec<SyncRow>,
    /// Enabled locally but absent from the export.
    pub extra: Vec<ModEntry>,
    /// Packs present on both sides load in a different order.
    pub order_differs: bool,
    pub ok: bool,
}

pub fn verify_export(rows: &[ExportRow], mods: &[ModEntry], enabled_in_order: &[String], local_hashes: &HashMap<String, String>) -> SyncResult {
    let enabled: HashSet<&str> = enabled_in_order.iter().map(String::as_str).collect();
    let out: Vec<SyncRow> = rows
        .iter()
        .map(|row| {
            let local = match_local(row, mods).cloned();
            let status = match &local {
                None => SyncStatus::Missing,
                Some(l) if !enabled.contains(l.key.as_str()) => SyncStatus::Disabled,
                Some(l) if row.size.map(|s| s != l.size).unwrap_or(false) => SyncStatus::Different,
                Some(l) => match (&row.sha256, local_hashes.get(&l.key)) {
                    (Some(theirs), Some(ours)) if theirs == ours => SyncStatus::Match,
                    (Some(_), Some(_)) => SyncStatus::Different,
                    _ => SyncStatus::Unverified,
                },
            };
            SyncRow { row: row.clone(), local, status }
        })
        .collect();
    let matched: Vec<String> = out.iter().filter_map(|r| r.local.as_ref()).filter(|l| enabled.contains(l.key.as_str())).map(|l| l.key.clone()).collect();
    let matched_set: HashSet<&str> = matched.iter().map(String::as_str).collect();
    let extra: Vec<ModEntry> = enabled_in_order.iter().filter(|k| !matched_set.contains(k.as_str())).filter_map(|k| mods.iter().find(|m| &m.key == k).cloned()).collect();
    let local_order: Vec<&String> = enabled_in_order.iter().filter(|k| matched_set.contains(k.as_str())).collect();
    let order_differs = local_order.iter().map(|s| s.as_str()).ne(matched.iter().map(String::as_str));
    let ok = out.iter().all(|r| r.status == SyncStatus::Match) && extra.is_empty() && !order_differs;
    SyncResult { rows: out, extra, order_differs, ok }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::PackType;

    fn m(key: &str, file: &str, size: u64, source: ModSource) -> ModEntry {
        ModEntry {
            key: key.into(),
            file: file.into(),
            path: file.into(),
            dir: String::new(),
            source,
            workshop_id: None,
            pack_type: PackType::Mod,
            size,
            mtime: 0,
            preview_path: None,
            installed_updated: None,
            latest_updated: None,
        }
    }

    fn h(c: char) -> String {
        c.to_string().repeat(64)
    }

    fn ph(key: &str, file: &str, size: u64, c: char) -> PackHash {
        PackHash { key: key.into(), file: file.into(), size, sha256: h(c) }
    }

    #[test]
    fn export_roundtrip_and_v1() {
        let text = build_export("MP", &[ph("ws:1/a.pack", "a.pack", 10, 'a'), ph("data:b c.pack", "b c.pack", 20, 'b')], "2026-09-18");
        let rows = parse_export(&text);
        assert_eq!(rows[0], ExportRow { key: "ws:1/a.pack".into(), file: "a.pack".into(), size: Some(10), sha256: Some(h('a')) });
        assert_eq!(rows[1].file, "b c.pack");
        let v1 = parse_export("# old\nws:1/a.pack\t# a.pack\ndata:x.pack\nnot a key\n");
        assert_eq!(v1, vec![
            ExportRow { key: "ws:1/a.pack".into(), file: "a.pack".into(), size: None, sha256: None },
            ExportRow { key: "data:x.pack".into(), file: "x.pack".into(), size: None, sha256: None },
        ]);
    }

    #[test]
    fn verify_statuses() {
        let mods = vec![m("ws:1/a.pack", "a.pack", 10, ModSource::Workshop), m("ext:z:/dev/b.pack", "b.pack", 20, ModSource::Folder), m("data:c.pack", "c.pack", 30, ModSource::Data)];
        let rows = parse_export(&build_export("MP", &[ph("ws:1/a.pack", "a.pack", 10, 'a'), ph("data:b.pack", "b.pack", 20, 'b')], "d"));
        let hashes = |a: char| -> HashMap<String, String> { [("ws:1/a.pack".to_string(), h(a)), ("ext:z:/dev/b.pack".to_string(), h('b'))].into_iter().collect() };

        let ok = verify_export(&rows, &mods, &["ws:1/a.pack".into(), "ext:z:/dev/b.pack".into()], &hashes('a'));
        assert!(ok.ok);

        let bad = verify_export(&rows, &mods, &["ext:z:/dev/b.pack".into(), "ws:1/a.pack".into(), "data:c.pack".into()], &hashes('f'));
        assert_eq!(bad.rows.iter().map(|r| r.status).collect::<Vec<_>>(), vec![SyncStatus::Different, SyncStatus::Match]);
        assert_eq!(bad.extra.iter().map(|m| m.file.as_str()).collect::<Vec<_>>(), vec!["c.pack"]);
        assert!(bad.order_differs && !bad.ok);

        assert_eq!(verify_export(&rows, &mods, &["ws:1/a.pack".into()], &hashes('a')).rows[1].status, SyncStatus::Disabled);
        assert_eq!(verify_export(&parse_export(&format!("ws:9/zz.pack\tzz.pack\t1\t{}", h('0'))), &mods, &[], &HashMap::new()).rows[0].status, SyncStatus::Missing);
        assert_eq!(verify_export(&parse_export(&format!("data:c.pack\tc.pack\t31\t{}", h('c'))), &mods, &["data:c.pack".into()], &HashMap::new()).rows[0].status, SyncStatus::Different);
    }
}
