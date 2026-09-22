//! Per-mod status (the dot in the mod list) and the script-extender requirement.
//!
//!   Pending  red    Steam has a newer version than the one installed
//!   Old      amber  last Workshop update predates the current game build (informational)
//!   Ok       green  up to date
//!   Unknown  grey   Workshop item without update data
//!   Local    grey   data/ or folder pack: nothing to compare against
//!
//! A mod needs the script extender exactly when its pack ships `SE/script_extender.json`
//! (see `se_scan`); there is no manual override.

use crate::fmt::format_date;
use crate::packs::{ModEntry, ModSource};
use crate::profiles::Profile;
use crate::se_scan::SeInfo;
use crate::workshop::WorkshopItem;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusKind {
    Pending,
    Old,
    Ok,
    Unknown,
    Local,
}

impl StatusKind {
    pub fn label(self) -> &'static str {
        match self {
            StatusKind::Pending => "Update pending",
            StatusKind::Old => "Older than game patch",
            StatusKind::Ok => "Up to date",
            StatusKind::Unknown => "Unknown",
            StatusKind::Local => "Local file",
        }
    }
    /// Sort weight: problems first.
    pub fn rank(self) -> u8 {
        match self {
            StatusKind::Pending => 0,
            StatusKind::Old => 1,
            StatusKind::Unknown => 2,
            StatusKind::Local => 3,
            StatusKind::Ok => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModStatus {
    pub kind: StatusKind,
    pub text: String,
    pub latest: Option<u64>,
    pub installed: Option<u64>,
}

pub fn mod_status(m: Option<&ModEntry>, ws: Option<&WorkshopItem>, cutoff: u64) -> ModStatus {
    let Some(m) = m else {
        return ModStatus { kind: StatusKind::Unknown, text: "Not installed".into(), latest: None, installed: None };
    };
    if let Some(nx) = m.nexus.as_ref().filter(|_| m.source == ModSource::Nexus) {
        return nexus_status(nx);
    }
    if m.source != ModSource::Workshop {
        let text = if m.source == ModSource::Data {
            "Local pack in data/: no Workshop version to compare"
        } else {
            "Pack from an extra folder: no Workshop version to compare"
        };
        return ModStatus { kind: StatusKind::Local, text: text.into(), latest: None, installed: None };
    }
    let api = ws.filter(|w| !w.from_launcher_cache && w.time_updated > 0).map(|w| w.time_updated).unwrap_or(0);
    let latest = Some(m.latest_updated.unwrap_or(0).max(api)).filter(|v| *v > 0);
    let installed = m.installed_updated;
    if let (Some(l), Some(i)) = (latest, installed) {
        if l > i {
            return ModStatus {
                kind: StatusKind::Pending,
                text: format!(
                    "Update pending: Steam has the {} version, {} is installed. Use \"Force update from Steam\" to download it now.",
                    format_date(l),
                    format_date(i)
                ),
                latest,
                installed,
            };
        }
    }
    let Some(published) = latest.or(installed) else {
        return ModStatus { kind: StatusKind::Unknown, text: "No update information yet (Steam has not reported this item)".into(), latest: None, installed };
    };
    if cutoff > 0 && published < cutoff {
        return ModStatus {
            kind: StatusKind::Old,
            text: format!("Last updated {}, before the current game build ({}). It may still work fine.", format_date(published), format_date(cutoff)),
            latest,
            installed,
        };
    }
    ModStatus { kind: StatusKind::Ok, text: format!("Up to date (updated {})", format_date(published)), latest, installed }
}

fn nexus_status(nx: &crate::nexus::NexusRef) -> ModStatus {
    let have = if nx.active.version.is_empty() { "unknown version".to_string() } else { format!("version {}", nx.active.version) };
    let installed = Some(nx.active.uploaded).filter(|v| *v > 0);
    if let Some(n) = &nx.newer {
        return ModStatus {
            kind: StatusKind::Pending,
            text: format!(
                "Update on Nexus Mods: {} ({}) is available, {have} is installed. Download it with \"Mod Manager Download\" on the mod's Files tab.",
                if n.version.is_empty() { n.name.clone() } else { format!("version {}", n.version) },
                format_date(n.uploaded)
            ),
            latest: Some(n.uploaded),
            installed,
        };
    }
    if nx.mod_id.is_none() {
        return ModStatus { kind: StatusKind::Local, text: "Installed from an archive: no Nexus page to compare".into(), latest: None, installed };
    }
    ModStatus { kind: StatusKind::Ok, text: format!("From Nexus Mods, {have}"), latest: installed, installed }
}

/// What a mod asks of the script extender.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeRequirement {
    pub required: bool,
    pub author: Option<String>,
    pub min_version: Option<String>,
    pub max_version: Option<String>,
    pub notes: Option<String>,
    pub error: Option<String>,
}

pub fn se_requirement(scan: Option<&SeInfo>) -> SeRequirement {
    match scan {
        Some(s) if s.required => SeRequirement {
            required: true,
            author: s.author.clone(),
            min_version: s.min_version.clone(),
            max_version: s.max_version.clone(),
            notes: s.notes.clone(),
            error: s.error.clone(),
        },
        _ => SeRequirement::default(),
    }
}

/// "0.28", "0.28 or newer", "up to 0.28", "0.26 – 0.28", or "".
pub fn range_text(r: &SeRequirement) -> String {
    match (&r.min_version, &r.max_version) {
        (Some(lo), Some(hi)) if lo == hi => lo.clone(),
        (Some(lo), Some(hi)) => format!("{lo} – {hi}"),
        (Some(lo), None) => format!("{lo} or newer"),
        (None, Some(hi)) => format!("up to {hi}"),
        (None, None) => String::new(),
    }
}

/// "Declared in SE/script_extender.json · by Ironic · version 0.28"
pub fn source_text(r: &SeRequirement) -> String {
    if !r.required {
        return "Does not use the script extender".into();
    }
    let mut bits = vec!["Declared in SE/script_extender.json".to_string()];
    if let Some(a) = &r.author {
        bits.push(format!("by {a}"));
    }
    let range = range_text(r);
    if !range.is_empty() {
        bits.push(format!("version {range}"));
    }
    bits.join(" · ")
}

fn parts(v: &str) -> Vec<u64> {
    v.trim().trim_start_matches(['v', 'V']).split(['.', '-']).map(|x| x.trim().parse().unwrap_or(0)).collect()
}

/// Dotted numeric compare; missing parts count as 0 (0.28 == 0.28.0).
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        match pa.get(i).unwrap_or(&0).cmp(pb.get(i).unwrap_or(&0)) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    Ordering::Equal
}

pub fn version_less(a: &str, b: &str) -> bool {
    compare_versions(a, b) == Ordering::Less
}

/// Is `have` newer than `max`, compared at the maximum's own precision (max 0.28 allows 0.28.x)?
pub fn version_above_max(have: &str, max: &str) -> bool {
    let n = parts(max).len();
    let h: Vec<String> = parts(have).into_iter().take(n).map(|x| x.to_string()).collect();
    compare_versions(&h.join("."), max) == Ordering::Greater
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SeUnmet {
    Off,
    NoDll,
    TooOld,
    TooNew,
}

/// Why this launch would not satisfy the requirement (None = satisfied or not required).
pub fn se_unmet(r: &SeRequirement, profile_dll: bool, dll_version: Option<&str>) -> Option<SeUnmet> {
    if !r.required {
        return None;
    }
    if !profile_dll {
        return Some(SeUnmet::Off);
    }
    let Some(have) = dll_version else { return Some(SeUnmet::NoDll) };
    if r.min_version.as_deref().map(|m| version_less(have, m)).unwrap_or(false) {
        return Some(SeUnmet::TooOld);
    }
    if r.max_version.as_deref().map(|m| version_above_max(have, m)).unwrap_or(false) {
        return Some(SeUnmet::TooNew);
    }
    None
}

pub fn unmet_text(u: SeUnmet, r: &SeRequirement, have: Option<&str>) -> String {
    let have = have.unwrap_or("?");
    match u {
        SeUnmet::Off => "The script extender is off for this profile.".into(),
        SeUnmet::NoDll => "No script extender DLL matches this game build.".into(),
        SeUnmet::TooOld => format!("Needs script extender {}; v{have} is installed.", range_text(r)),
        SeUnmet::TooNew => format!("Supports script extender {} only; v{have} is installed.", range_text(r)),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeProblem {
    pub kind: SeUnmet,
    pub mods: Vec<String>,
    pub text: String,
}

/// Enabled mods whose script-extender need this launch would not meet, grouped by reason.
pub fn se_problems(profile: &Profile, requirement: impl Fn(&str) -> SeRequirement, title: impl Fn(&str) -> String, dll_version: Option<&str>) -> Vec<SeProblem> {
    let mut out: Vec<SeProblem> = Vec::new();
    for e in profile.entries.iter().filter(|e| e.enabled && !e.is_separator()) {
        let r = requirement(&e.key);
        let Some(u) = se_unmet(&r, profile.dll, dll_version) else { continue };
        let text = unmet_text(u, &r, dll_version);
        match out.iter_mut().find(|p| p.kind == u) {
            Some(p) => {
                p.mods.push(title(&e.key));
                if p.text != text {
                    p.text = match u {
                        SeUnmet::TooOld => format!("Some mods need a newer script extender than v{}.", dll_version.unwrap_or("?")),
                        SeUnmet::TooNew => format!("Some mods do not support script extender v{}.", dll_version.unwrap_or("?")),
                        _ => text,
                    };
                }
            }
            None => out.push(SeProblem { kind: u, mods: vec![title(&e.key)], text }),
        }
    }
    let order = [SeUnmet::Off, SeUnmet::NoDll, SeUnmet::TooOld, SeUnmet::TooNew];
    out.sort_by_key(|p| order.iter().position(|o| *o == p.kind));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::PackType;
    use crate::profiles::ProfileEntry;

    fn ws(t: u64, seed: bool) -> WorkshopItem {
        WorkshopItem { id: "1".into(), time_updated: t, from_launcher_cache: seed, ..Default::default() }
    }

    fn m(source: ModSource, installed: Option<u64>, latest: Option<u64>) -> ModEntry {
        ModEntry {
            key: "ws:1/a.pack".into(),
            file: "a.pack".into(),
            path: String::new(),
            dir: String::new(),
            source,
            workshop_id: Some("1".into()),
            pack_type: PackType::Mod,
            size: 0,
            mtime: 0,
            preview_path: None,
            installed_updated: installed,
            latest_updated: latest,
            nexus: None,
        }
    }

    #[test]
    fn status_kinds() {
        let w = ModSource::Workshop;
        assert_eq!(mod_status(Some(&m(w, Some(1000), Some(1500))), None, 500).kind, StatusKind::Pending);
        assert_eq!(mod_status(Some(&m(w, Some(1000), Some(1000))), Some(&ws(2000, false)), 500).kind, StatusKind::Pending);
        assert_eq!(mod_status(Some(&m(w, Some(1000), Some(1000))), Some(&ws(2000, true)), 500).kind, StatusKind::Ok);
        assert_eq!(mod_status(Some(&m(w, Some(400), Some(400))), None, 500).kind, StatusKind::Old);
        assert_eq!(mod_status(Some(&m(w, Some(400), Some(400))), None, 0).kind, StatusKind::Ok);
        assert_eq!(mod_status(Some(&m(w, None, None)), None, 500).kind, StatusKind::Unknown);
        assert_eq!(mod_status(Some(&m(ModSource::Data, None, None)), None, 500).kind, StatusKind::Local);
        assert_eq!(mod_status(Some(&m(ModSource::Folder, None, None)), None, 500).kind, StatusKind::Local);
        assert_eq!(mod_status(None, None, 500).kind, StatusKind::Unknown);
    }

    #[test]
    fn nexus_status_kinds() {
        use crate::nexus::{InstalledFile, NexusRef, RemoteFile};
        let mut e = m(ModSource::Nexus, None, None);
        let nx = NexusRef {
            slot: "7".into(),
            mod_id: Some(7),
            mod_name: "M".into(),
            author: String::new(),
            active: InstalledFile { version: "1.0".into(), uploaded: 100, ..Default::default() },
            versions: Vec::new(),
            newer: None,
        };
        e.nexus = Some(nx.clone());
        assert_eq!(mod_status(Some(&e), None, 500).kind, StatusKind::Ok);
        e.nexus = Some(NexusRef { newer: Some(RemoteFile { version: "2.0".into(), uploaded: 200, ..Default::default() }), ..nx.clone() });
        let st = mod_status(Some(&e), None, 500);
        assert_eq!(st.kind, StatusKind::Pending);
        assert!(st.text.contains("2.0"));
        e.nexus = Some(NexusRef { mod_id: None, ..nx });
        assert_eq!(mod_status(Some(&e), None, 500).kind, StatusKind::Local);
    }

    fn info(min: Option<&str>, max: Option<&str>) -> SeInfo {
        SeInfo {
            required: true,
            path: "SE/script_extender.json".into(),
            author: Some("Ironic".into()),
            min_version: min.map(str::to_owned),
            max_version: max.map(str::to_owned),
            notes: None,
            error: None,
        }
    }

    #[test]
    fn requirement_comes_only_from_the_manifest() {
        assert!(se_requirement(Some(&info(Some("0.28"), None))).required);
        assert!(!se_requirement(None).required);
        assert!(!se_requirement(Some(&SeInfo::default())).required);
        assert_eq!(source_text(&se_requirement(Some(&info(Some("0.28"), Some("0.28"))))), "Declared in SE/script_extender.json · by Ironic · version 0.28");
    }

    #[test]
    fn ranges_and_versions() {
        let r = |a, b| se_requirement(Some(&info(a, b)));
        assert_eq!(range_text(&r(Some("0.28"), Some("0.28"))), "0.28");
        assert_eq!(range_text(&r(Some("0.26"), Some("0.28"))), "0.26 – 0.28");
        assert_eq!(range_text(&r(Some("0.28"), None)), "0.28 or newer");
        assert_eq!(range_text(&r(None, Some("0.28"))), "up to 0.28");
        assert!(version_less("0.3", "0.28"));
        assert!(version_less("0.9.0", "0.23.0"));
        assert!(!version_less("0.28.0", "0.28"));
        assert!(!version_above_max("0.28.5", "0.28"));
        assert!(version_above_max("0.29.0", "0.28"));
        assert!(version_above_max("0.28.6", "0.28.5"));
        assert!(!version_above_max("0.30.0", "0.30"));
    }

    #[test]
    fn unmet_order() {
        let r = se_requirement(Some(&info(Some("0.28"), Some("0.28"))));
        assert_eq!(se_unmet(&r, false, Some("0.28.0")), Some(SeUnmet::Off));
        assert_eq!(se_unmet(&r, true, None), Some(SeUnmet::NoDll));
        assert_eq!(se_unmet(&r, true, Some("0.27.9")), Some(SeUnmet::TooOld));
        assert_eq!(se_unmet(&r, true, Some("0.29.0")), Some(SeUnmet::TooNew));
        assert_eq!(se_unmet(&r, true, Some("0.28.3")), None);
        assert_eq!(se_unmet(&SeRequirement::default(), false, None), None);
    }

    #[test]
    fn problems_grouped() {
        let e = |k: &str, on: bool| ProfileEntry { key: k.into(), enabled: on, label: None, collapsed: false };
        let prof = |dll: bool| Profile { name: "p".into(), entries: vec![e("a", true), e("b", true), e("c", false), e("d", true)], dll, skip_intro: false, last_played: None };
        let req = |k: &str| match k {
            "a" => se_requirement(Some(&info(Some("0.28"), Some("0.28")))),
            "c" => se_requirement(Some(&info(None, None))),
            "d" => se_requirement(Some(&info(None, Some("0.24")))),
            _ => SeRequirement::default(),
        };
        let title = |k: &str| k.to_uppercase();
        let off = se_problems(&prof(false), req, title, Some("0.28.0"));
        assert_eq!(off.len(), 1);
        assert_eq!((off[0].kind, off[0].mods.clone()), (SeUnmet::Off, vec!["A".to_string(), "D".to_string()]));
        assert_eq!(se_problems(&prof(true), req, title, None)[0].kind, SeUnmet::NoDll);
        let mixed = se_problems(&prof(true), req, title, Some("0.25.0"));
        assert_eq!(mixed.iter().map(|p| (p.kind, p.mods.clone())).collect::<Vec<_>>(), vec![(SeUnmet::TooOld, vec!["A".to_string()]), (SeUnmet::TooNew, vec!["D".to_string()])]);
        assert_eq!(mixed[0].text, "Needs script extender 0.28; v0.25.0 is installed.");
    }
}
