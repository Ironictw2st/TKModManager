//! Operations on profiles and the profiles document (what the UI does when the user clicks):
//! reconcile with the installed packs, toggle, move, sort, create/rename/delete.

use crate::fmt::compare_pack_names;
use crate::packs::ModEntry;
use crate::profiles::{Profile, ProfileEntry, ProfilesDoc};
use crate::workshop::WorkshopItem;
use std::collections::{HashMap, HashSet};

fn entry(key: &str, enabled: bool) -> ProfileEntry {
    ProfileEntry { key: key.to_string(), enabled, label: None, collapsed: false }
}

/// Make sure every installed pack has an entry: new packs are appended disabled, in file-name
/// order. Entries for packs that are gone stay (shown as missing). Returns true when changed.
pub fn reconcile(profile: &mut Profile, mods: &[ModEntry]) -> bool {
    let have: HashSet<&str> = profile.entries.iter().map(|e| e.key.as_str()).collect();
    let mut missing: Vec<&ModEntry> = mods.iter().filter(|m| !have.contains(m.key.as_str())).collect();
    if missing.is_empty() {
        return false;
    }
    missing.sort_by(|a, b| compare_pack_names(&a.file, &b.file));
    profile.entries.extend(missing.into_iter().map(|m| entry(&m.key, false)));
    true
}

pub fn reconcile_all(doc: &mut ProfilesDoc, mods: &[ModEntry]) -> bool {
    let mut changed = false;
    for p in &mut doc.profiles {
        changed |= reconcile(p, mods);
    }
    changed
}

/// Apply `f` to each run of pack entries between separators; separators keep their place.
pub fn map_segments(entries: &[ProfileEntry], mut f: impl FnMut(Vec<ProfileEntry>) -> Vec<ProfileEntry>) -> Vec<ProfileEntry> {
    let mut out = Vec::with_capacity(entries.len());
    let mut seg = Vec::new();
    for e in entries {
        if e.is_separator() {
            out.extend(f(std::mem::take(&mut seg)));
            out.push(e.clone());
        } else {
            seg.push(e.clone());
        }
    }
    out.extend(f(seg));
    out
}

/// Has this pack changed since the profile was last launched (Workshop update or file time)?
pub fn updated_since(last_played: Option<u64>, m: Option<&ModEntry>, ws: Option<&WorkshopItem>) -> bool {
    match (last_played, m) {
        (Some(t), Some(m)) if t > 0 => m.mtime.max(ws.map(|w| w.time_updated).unwrap_or(0)) > t,
        _ => false,
    }
}

/// Set (or flip, when `enabled` is None) the given pack entries. Separators are ignored.
pub fn toggle(entries: &mut [ProfileEntry], keys: &[String], enabled: Option<bool>) {
    let ks: HashSet<&str> = keys.iter().map(String::as_str).collect();
    for e in entries.iter_mut().filter(|e| !e.is_separator() && ks.contains(e.key.as_str())) {
        e.enabled = enabled.unwrap_or(!e.enabled);
    }
}

/// Alphabetical by pack file name inside each group.
pub fn sort_alpha(entries: &[ProfileEntry], file_of: &HashMap<String, String>) -> Vec<ProfileEntry> {
    map_segments(entries, |mut seg| {
        seg.sort_by(|a, b| {
            let fa = file_of.get(&a.key).map(String::as_str).unwrap_or(&a.key);
            let fb = file_of.get(&b.key).map(String::as_str).unwrap_or(&b.key);
            compare_pack_names(fa, fb)
        });
        seg
    })
}

/// Enabled packs first, inside each group (stable).
pub fn enabled_to_top(entries: &[ProfileEntry]) -> Vec<ProfileEntry> {
    map_segments(entries, |seg| {
        let (on, off): (Vec<_>, Vec<_>) = seg.into_iter().partition(|e| e.enabled);
        on.into_iter().chain(off).collect()
    })
}

/// Move the selected pack entries one step up (-1) or down (+1) as a block; separators are not
/// moved by this. Returns false when already at the edge.
pub fn nudge(entries: &mut Vec<ProfileEntry>, keys: &[String], dir: i32) -> bool {
    let ks: HashSet<&str> = keys.iter().map(String::as_str).filter(|k| !crate::groups::is_separator_key(k)).collect();
    let idx: Vec<usize> = entries.iter().enumerate().filter(|(_, e)| ks.contains(e.key.as_str())).map(|(i, _)| i).collect();
    let (Some(&lo), Some(&hi)) = (idx.first(), idx.last()) else { return false };
    let target = if dir < 0 { lo.checked_sub(1) } else { Some(hi + 1) };
    let Some(target) = target.filter(|t| *t < entries.len()) else { return false };
    let target_key = entries[target].key.clone();
    let moving: Vec<ProfileEntry> = entries.iter().filter(|e| ks.contains(e.key.as_str())).cloned().collect();
    let rest: Vec<ProfileEntry> = entries.iter().filter(|e| !ks.contains(e.key.as_str())).cloned().collect();
    let Some(pos) = rest.iter().position(|e| e.key == target_key) else { return false };
    let at = if dir < 0 { pos } else { pos + 1 };
    let mut out = rest[..at].to_vec();
    out.extend(moving);
    out.extend_from_slice(&rest[at..]);
    *entries = out;
    true
}

/// Put `keys` first (enabled, in that order), everything else after (keeping its state/order).
/// Used by every import (CA list, collection, text).
pub fn enable_first(entries: &[ProfileEntry], keys: &[String]) -> Vec<ProfileEntry> {
    let ks: HashSet<&str> = keys.iter().map(String::as_str).collect();
    let mut out: Vec<ProfileEntry> = keys.iter().map(|k| entry(k, true)).collect();
    out.extend(entries.iter().filter(|e| !ks.contains(e.key.as_str())).cloned());
    out
}

pub fn active(doc: &ProfilesDoc) -> Option<&Profile> {
    doc.profiles.iter().find(|p| p.name == doc.active).or_else(|| doc.profiles.first())
}

pub fn active_mut(doc: &mut ProfilesDoc) -> Option<&mut Profile> {
    let name = doc.active.clone();
    let idx = doc.profiles.iter().position(|p| p.name == name).unwrap_or(0);
    doc.profiles.get_mut(idx)
}

/// New profile, becomes active. `from` = duplicate; otherwise every installed pack, disabled.
pub fn create(doc: &mut ProfilesDoc, name: &str, from: Option<&Profile>, mods: &[ModEntry]) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("The profile name is empty".into());
    }
    if doc.profiles.iter().any(|p| p.name == name) {
        return Err(format!("A profile named \"{name}\" already exists"));
    }
    let profile = match from {
        Some(p) => Profile { name: name.into(), last_played: None, ..p.clone() },
        None => {
            let mut p = Profile { name: name.into(), entries: vec![], dll: false, skip_intro: false, last_played: None };
            reconcile(&mut p, mods);
            p
        }
    };
    doc.profiles.push(profile);
    doc.active = name.into();
    Ok(())
}

pub fn rename(doc: &mut ProfilesDoc, old: &str, new: &str) -> Result<(), String> {
    let new = new.trim();
    if new.is_empty() {
        return Err("The profile name is empty".into());
    }
    if old != new && doc.profiles.iter().any(|p| p.name == new) {
        return Err(format!("A profile named \"{new}\" already exists"));
    }
    let p = doc.profiles.iter_mut().find(|p| p.name == old).ok_or("No such profile")?;
    p.name = new.into();
    if doc.active == old {
        doc.active = new.into();
    }
    Ok(())
}

pub fn delete(doc: &mut ProfilesDoc, name: &str) -> Result<(), String> {
    if doc.profiles.len() <= 1 {
        return Err("Cannot delete the last profile".into());
    }
    doc.profiles.retain(|p| p.name != name);
    if doc.active == name {
        doc.active = doc.profiles[0].name.clone();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::{ModSource, PackType};

    fn m(key: &str, file: &str, mtime: u64) -> ModEntry {
        ModEntry {
            key: key.into(),
            file: file.into(),
            path: file.into(),
            dir: String::new(),
            source: ModSource::Data,
            workshop_id: None,
            pack_type: PackType::Mod,
            size: 0,
            mtime,
            preview_path: None,
            installed_updated: None,
            latest_updated: None,
        }
    }

    fn p(entries: Vec<ProfileEntry>) -> Profile {
        Profile { name: "x".into(), entries, dll: false, skip_intro: false, last_played: None }
    }

    fn keys(v: &[ProfileEntry]) -> Vec<&str> {
        v.iter().map(|e| e.key.as_str()).collect()
    }

    #[test]
    fn reconcile_appends_new_packs_sorted_and_keeps_missing() {
        let mut prof = p(vec![entry("data:z.pack", true), entry("sep:1", false), entry("ws:1/gone.pack", true)]);
        assert!(reconcile(&mut prof, &[m("data:b.pack", "b.pack", 0), m("data:!a.pack", "!a.pack", 0), m("data:z.pack", "z.pack", 0)]));
        assert_eq!(keys(&prof.entries), vec!["data:z.pack", "sep:1", "ws:1/gone.pack", "data:!a.pack", "data:b.pack"]);
        assert!(!prof.entries[3].enabled);
        assert!(!reconcile(&mut prof, &[m("data:z.pack", "z.pack", 0)]));
    }

    #[test]
    fn segment_sorting_and_enabled_to_top() {
        let v = vec![entry("data:b", false), entry("data:a", true), entry("sep:1", false), entry("data:d", false), entry("data:c", true)];
        let files: HashMap<String, String> = [("data:a", "a"), ("data:b", "b"), ("data:c", "c"), ("data:d", "d")].iter().map(|(k, f)| (k.to_string(), f.to_string())).collect();
        assert_eq!(keys(&sort_alpha(&v, &files)), vec!["data:a", "data:b", "sep:1", "data:c", "data:d"]);
        assert_eq!(keys(&enabled_to_top(&v)), vec!["data:a", "data:b", "sep:1", "data:c", "data:d"]);
    }

    #[test]
    fn updated_since_uses_newest_time() {
        let ws = WorkshopItem { time_updated: 200, ..Default::default() };
        assert!(!updated_since(None, Some(&m("k", "f", 999)), None));
        assert!(!updated_since(Some(100), Some(&m("k", "f", 50)), None));
        assert!(updated_since(Some(100), Some(&m("k", "f", 150)), None));
        assert!(updated_since(Some(100), Some(&m("k", "f", 50)), Some(&ws)));
        assert!(!updated_since(Some(100), None, Some(&ws)));
    }

    #[test]
    fn toggle_nudge_and_enable_first() {
        let mut v = vec![entry("a", false), entry("sep:1", false), entry("b", true), entry("c", false)];
        toggle(&mut v, &["a".into(), "sep:1".into(), "b".into()], None);
        assert_eq!(v.iter().map(|e| e.enabled).collect::<Vec<_>>(), vec![true, false, false, false]);
        assert!(nudge(&mut v, &["c".into()], -1));
        assert_eq!(keys(&v), vec!["a", "sep:1", "c", "b"]);
        assert!(!nudge(&mut v, &["a".into()], -1));
        assert_eq!(keys(&enable_first(&v, &["b".into(), "x".into()])), vec!["b", "x", "a", "sep:1", "c"]);
    }

    #[test]
    fn profile_document_operations() {
        let mut doc = ProfilesDoc::default();
        create(&mut doc, "MP", None, &[m("data:a.pack", "a.pack", 0)]).unwrap();
        assert_eq!(doc.active, "MP");
        assert_eq!(active(&doc).unwrap().entries.len(), 1);
        assert!(create(&mut doc, "MP", None, &[]).is_err());
        let copy = active(&doc).cloned();
        create(&mut doc, "MP copy", copy.as_ref(), &[]).unwrap();
        rename(&mut doc, "MP copy", "Coop").unwrap();
        assert_eq!(doc.active, "Coop");
        assert!(rename(&mut doc, "Coop", "MP").is_err());
        delete(&mut doc, "Coop").unwrap();
        assert_eq!(doc.active, "Default");
        delete(&mut doc, "MP").unwrap();
        assert!(delete(&mut doc, "Default").is_err());
    }
}
