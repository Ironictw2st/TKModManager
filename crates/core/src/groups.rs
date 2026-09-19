//! Separators double as group headers (MO2 style): a separator entry heads every pack entry that
//! follows it, up to the next separator. All functions are pure over the entries list.

use crate::profiles::{ProfileEntry, SEPARATOR_PREFIX};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

pub fn is_separator_key(key: &str) -> bool {
    key.starts_with(SEPARATOR_PREFIX)
}

pub fn new_separator_key() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{SEPARATOR_PREFIX}{nanos:x}-{:x}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// Keys of the pack entries that belong to separator `sep_key`.
pub fn group_members(entries: &[ProfileEntry], sep_key: &str) -> Vec<String> {
    let Some(start) = entries.iter().position(|e| e.key == sep_key) else { return vec![] };
    entries[start + 1..].iter().take_while(|e| !e.is_separator()).map(|e| e.key.clone()).collect()
}

/// The separator heading `key`, if any.
pub fn group_of(entries: &[ProfileEntry], key: &str) -> Option<String> {
    let idx = entries.iter().position(|e| e.key == key)?;
    entries[..=idx].iter().rev().find(|e| e.is_separator()).map(|e| e.key.clone())
}

/// Insert a new separator before `before_key` (or at the end). Returns the new key.
pub fn add_separator(entries: &mut Vec<ProfileEntry>, label: &str, before_key: Option<&str>) -> String {
    let key = new_separator_key();
    let sep = ProfileEntry { key: key.clone(), enabled: false, label: Some(label.to_string()), collapsed: false };
    match before_key.and_then(|b| entries.iter().position(|e| e.key == b)) {
        Some(at) => entries.insert(at, sep),
        None => entries.push(sep),
    }
    key
}

/// Remove the separator only; its members stay where they are (joining the group above).
pub fn remove_separator(entries: &mut Vec<ProfileEntry>, sep_key: &str) {
    entries.retain(|e| e.key != sep_key);
}

pub fn rename_separator(entries: &mut [ProfileEntry], sep_key: &str, label: &str) {
    if let Some(e) = entries.iter_mut().find(|e| e.key == sep_key) {
        e.label = Some(label.to_string());
    }
}

pub fn set_collapsed(entries: &mut [ProfileEntry], sep_key: &str, collapsed: bool) {
    if let Some(e) = entries.iter_mut().find(|e| e.key == sep_key) {
        e.collapsed = collapsed;
    }
}

/// Enable/disable every member of a group.
pub fn toggle_group(entries: &mut [ProfileEntry], sep_key: &str, enabled: bool) {
    let members: HashSet<String> = group_members(entries, sep_key).into_iter().collect();
    for e in entries.iter_mut().filter(|e| members.contains(&e.key)) {
        e.enabled = enabled;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupState {
    All,
    None,
    Some,
    Empty,
}

/// State of the header checkbox; `exists` filters out packs that are no longer installed.
pub fn group_state(entries: &[ProfileEntry], sep_key: &str, exists: impl Fn(&str) -> bool) -> GroupState {
    let members: Vec<String> = group_members(entries, sep_key).into_iter().filter(|k| exists(k)).collect();
    if members.is_empty() {
        return GroupState::Empty;
    }
    let on = members.iter().filter(|k| entries.iter().any(|e| &e.key == *k && e.enabled)).count();
    match on {
        0 => GroupState::None,
        n if n == members.len() => GroupState::All,
        _ => GroupState::Some,
    }
}

/// Move `keys` as a block so it lands before/after `over_key`. A dragged separator takes its
/// whole group along. Returns false when nothing moved.
pub fn move_block(entries: &mut Vec<ProfileEntry>, keys: &[String], over_key: &str) -> bool {
    let mut expanded: HashSet<String> = HashSet::new();
    for k in keys {
        expanded.insert(k.clone());
        if is_separator_key(k) {
            expanded.extend(group_members(entries, k));
        }
    }
    if expanded.contains(over_key) {
        return false;
    }
    let (Some(from), Some(to)) = (entries.iter().position(|e| expanded.contains(&e.key)), entries.iter().position(|e| e.key == over_key)) else {
        return false;
    };
    let moving: Vec<ProfileEntry> = entries.iter().filter(|e| expanded.contains(&e.key)).cloned().collect();
    let rest: Vec<ProfileEntry> = entries.iter().filter(|e| !expanded.contains(&e.key)).cloned().collect();
    let Some(mut at) = rest.iter().position(|e| e.key == over_key) else { return false };
    if from < to {
        at += 1;
        // Moving a group down onto a separator lands after that separator's whole group.
        if is_separator_key(over_key) && keys.iter().any(|k| is_separator_key(k)) {
            while at < rest.len() && !rest[at].is_separator() {
                at += 1;
            }
        }
    }
    let mut out = rest[..at].to_vec();
    out.extend(moving);
    out.extend_from_slice(&rest[at..]);
    *entries = out;
    true
}

/// Keys hidden because their separator is collapsed.
pub fn collapsed_keys(entries: &[ProfileEntry]) -> HashSet<String> {
    let mut hidden = HashSet::new();
    let mut collapsed = false;
    for e in entries {
        if e.is_separator() {
            collapsed = e.collapsed;
        } else if collapsed {
            hidden.insert(e.key.clone());
        }
    }
    hidden
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(key: &str, enabled: bool) -> ProfileEntry {
        ProfileEntry { key: key.into(), enabled, label: key.starts_with("sep:").then(|| key[4..].to_string()), collapsed: false }
    }

    fn base() -> Vec<ProfileEntry> {
        vec![e("data:top.pack", true), e("sep:A", false), e("data:a1.pack", true), e("data:a2.pack", false), e("sep:B", false), e("data:b1.pack", false)]
    }

    fn keys(v: &[ProfileEntry]) -> Vec<&str> {
        v.iter().map(|e| e.key.as_str()).collect()
    }

    #[test]
    fn members_and_owners() {
        assert_eq!(group_members(&base(), "sep:A"), vec!["data:a1.pack", "data:a2.pack"]);
        assert_eq!(group_members(&base(), "sep:B"), vec!["data:b1.pack"]);
        assert_eq!(group_of(&base(), "data:a2.pack").as_deref(), Some("sep:A"));
        assert_eq!(group_of(&base(), "data:top.pack"), None);
    }

    #[test]
    fn toggle_and_state() {
        let all = |_: &str| true;
        let mut v = base();
        assert_eq!(group_state(&v, "sep:A", all), GroupState::Some);
        toggle_group(&mut v, "sep:A", true);
        assert_eq!(group_state(&v, "sep:A", all), GroupState::All);
        assert!(!v.iter().find(|x| x.key == "data:b1.pack").unwrap().enabled);
        toggle_group(&mut v, "sep:A", false);
        assert_eq!(group_state(&v, "sep:A", all), GroupState::None);
        assert_eq!(group_state(&base(), "sep:A", |_| false), GroupState::Empty);
    }

    #[test]
    fn add_and_remove() {
        let mut v = base();
        let k = add_separator(&mut v, "New", Some("data:b1.pack"));
        assert_eq!(keys(&v), vec!["data:top.pack", "sep:A", "data:a1.pack", "data:a2.pack", "sep:B", k.as_str(), "data:b1.pack"]);
        let mut v = base();
        remove_separator(&mut v, "sep:B");
        assert_eq!(group_members(&v, "sep:A"), vec!["data:a1.pack", "data:a2.pack", "data:b1.pack"]);
    }

    #[test]
    fn moves_groups_as_blocks() {
        let mut v = base();
        assert!(move_block(&mut v, &["sep:A".into()], "data:b1.pack"));
        assert_eq!(keys(&v), vec!["data:top.pack", "sep:B", "data:b1.pack", "sep:A", "data:a1.pack", "data:a2.pack"]);
        let mut v = base();
        move_block(&mut v, &["sep:B".into()], "data:top.pack");
        assert_eq!(keys(&v), vec!["sep:B", "data:b1.pack", "data:top.pack", "sep:A", "data:a1.pack", "data:a2.pack"]);
    }

    #[test]
    fn moves_packs_between_groups() {
        let mut v = base();
        move_block(&mut v, &["data:b1.pack".into()], "data:a1.pack");
        assert_eq!(group_members(&v, "sep:A"), vec!["data:b1.pack", "data:a1.pack", "data:a2.pack"]);
        let mut v = base();
        assert!(!move_block(&mut v, &["sep:A".into()], "data:a1.pack"));
        assert_eq!(keys(&v), keys(&base()));
    }

    #[test]
    fn collapsed_members() {
        let mut v = base();
        set_collapsed(&mut v, "sep:A", true);
        let mut hidden: Vec<String> = collapsed_keys(&v).into_iter().collect();
        hidden.sort();
        assert_eq!(hidden, vec!["data:a1.pack", "data:a2.pack"]);
    }
}
