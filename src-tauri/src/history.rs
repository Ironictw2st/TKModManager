//! Launch history (`launch_history.json`): what was enabled at each launch and how the game
//! exited. Drives the crash helper: after an abnormal exit we show what changed since the last
//! launch that ended cleanly.

use crate::json_store;
use crate::packs::ModEntry;
use crate::paths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const KEEP: usize = 30;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PackSnap {
    pub key: String,
    pub file: String,
    pub size: u64,
    pub mtime: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRecord {
    /// Start time in unix milliseconds; doubles as the record id.
    pub id: u64,
    pub profile: String,
    pub started: u64,
    #[serde(default)]
    pub ended: Option<u64>,
    #[serde(default)]
    pub exit_code: Option<u32>,
    /// Enabled packs in load order.
    pub packs: Vec<PackSnap>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HistoryDoc {
    #[serde(default)]
    pub launches: Vec<LaunchRecord>,
}

fn path() -> PathBuf {
    paths::app_data_dir().join("launch_history.json")
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn snapshot(enabled: &[&ModEntry]) -> Vec<PackSnap> {
    enabled
        .iter()
        .map(|m| PackSnap { key: m.key.clone(), file: m.file.clone(), size: m.size, mtime: m.mtime })
        .collect()
}

/// Record a launch that just started; returns its id.
pub fn begin(profile: &str, packs: Vec<PackSnap>) -> u64 {
    let id = now_ms();
    let mut doc: HistoryDoc = json_store::load(&path()).unwrap_or_default();
    doc.launches.push(LaunchRecord { id, profile: profile.into(), started: id / 1000, ended: None, exit_code: None, packs });
    let drop = doc.launches.len().saturating_sub(KEEP);
    doc.launches.drain(..drop);
    let _ = json_store::save(&path(), &doc);
    id
}

pub fn finish(id: u64, exit_code: Option<u32>) {
    let mut doc: HistoryDoc = json_store::load(&path()).unwrap_or_default();
    if let Some(r) = doc.launches.iter_mut().find(|r| r.id == id) {
        r.ended = Some(now_ms() / 1000);
        r.exit_code = exit_code;
        let _ = json_store::save(&path(), &doc);
    }
}

/// What differs between two launches' enabled sets.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Diff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// Same pack, different file size or modified time (updated / rebuilt).
    pub changed: Vec<String>,
    /// Packs present in both launches load in a different relative order.
    pub reordered: bool,
}

pub fn diff(prev: &[PackSnap], cur: &[PackSnap]) -> Diff {
    let mut d = Diff::default();
    for c in cur {
        match prev.iter().find(|p| p.key == c.key) {
            None => d.added.push(c.file.clone()),
            Some(p) if p.size != c.size || p.mtime != c.mtime => d.changed.push(c.file.clone()),
            Some(_) => {}
        }
    }
    for p in prev {
        if !cur.iter().any(|c| c.key == p.key) {
            d.removed.push(p.file.clone());
        }
    }
    let common_prev: Vec<&str> = prev.iter().filter(|p| cur.iter().any(|c| c.key == p.key)).map(|p| p.key.as_str()).collect();
    let common_cur: Vec<&str> = cur.iter().filter(|c| prev.iter().any(|p| p.key == c.key)).map(|c| c.key.as_str()).collect();
    d.reordered = common_prev != common_cur;
    d
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub record: LaunchRecord,
    /// The last earlier launch that exited with code 0, if any.
    pub baseline: Option<LaunchRecord>,
    pub diff: Option<Diff>,
}

#[tauri::command]
pub fn launch_history() -> Vec<LaunchRecord> {
    let mut doc: HistoryDoc = json_store::load(&path()).unwrap_or_default();
    doc.launches.reverse();
    doc.launches
}

/// The record `id` (or the most recent launch when `id` is None) compared with the last clean
/// launch before it.
#[tauri::command]
pub fn crash_report(id: Option<u64>) -> Option<CrashReport> {
    let doc: HistoryDoc = json_store::load(&path()).unwrap_or_default();
    let idx = match id {
        Some(id) => doc.launches.iter().position(|r| r.id == id)?,
        None => doc.launches.len().checked_sub(1)?,
    };
    let record = doc.launches[idx].clone();
    let baseline = doc.launches[..idx].iter().rev().find(|r| r.exit_code == Some(0)).cloned();
    let diff = baseline.as_ref().map(|b| diff(&b.packs, &record.packs));
    Some(CrashReport { record, baseline, diff })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(key: &str, size: u64) -> PackSnap {
        PackSnap { key: key.into(), file: key.trim_start_matches("data:").into(), size, mtime: 1 }
    }

    #[test]
    fn diff_reports_added_removed_changed_and_order() {
        let prev = vec![s("data:a.pack", 1), s("data:b.pack", 1), s("data:c.pack", 1)];
        let cur = vec![s("data:b.pack", 1), s("data:a.pack", 2), s("data:d.pack", 1)];
        let d = diff(&prev, &cur);
        assert_eq!(d.added, vec!["d.pack"]);
        assert_eq!(d.removed, vec!["c.pack"]);
        assert_eq!(d.changed, vec!["a.pack"]);
        assert!(d.reordered);
    }

    #[test]
    fn identical_sets_have_empty_diff() {
        let a = vec![s("data:a.pack", 1), s("data:b.pack", 1)];
        assert_eq!(diff(&a, &a), Diff::default());
    }
}
