//! `script_extender.cfg`: the script extender's settings, read by the DLL at injection
//! (TK-ScriptExtender HANDOFF.md §2a, state of 0.43.0).
//!
//! Format: one `key=value` per line, `#` starts a comment, whitespace and surrounding quotes are
//! trimmed, a missing key means its default. The DLL reads the file from its own version folder
//! first, then from `dll\` above it; the manager writes the `dll\` one before each launch from
//! the profile's settings (or the default set in Settings).
//!
//! Every `Group::Sim` key feeds the multiplayer version lock (`[se <version>.<sync>]` in the
//! build text): players only see each other's lobbies with identical values. Three of them are
//! hashed by their literal text rather than their meaning, so the file always spells out every
//! simulation key, defaults included. Keys this table does not know (from a newer DLL) are kept.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// Main-menu build text.
    Menu,
    /// Changes the simulation: part of the multiplayer lock.
    Sim,
    /// Local only; may differ between players.
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `0` / `1`.
    Bool,
    Int { min: i64, max: i64 },
    /// One of the listed values (value, text).
    Choice(&'static [(&'static str, &'static str)]),
    /// Free text on one line.
    Text,
}

#[derive(Clone, Copy, Debug)]
pub struct KeyDef {
    pub key: &'static str,
    pub group: Group,
    pub kind: Kind,
    pub default: &'static str,
    pub label: &'static str,
    pub help: &'static str,
}

const CACHE_MODES: &[(&str, &str)] = &[("0", "Off"), ("1", "On"), ("2", "Verify only")];

/// Every key the manager knows, in the order they are written.
pub const KEYS: &[KeyDef] = &[
    KeyDef { key: "build_number", group: Group::Menu, kind: Kind::Text, default: "{game}", label: "Build text", help: "Main-menu build text. {game} = the game's own text, {version} = script extender version, {sync} = multiplayer settings hash. The [se version.sync] tag is always added." },
    KeyDef { key: "build_number_short", group: Group::Menu, kind: Kind::Text, default: "{game}", label: "Short build text", help: "Short form of the build text, same placeholders." },
    KeyDef { key: "build_modified", group: Group::Menu, kind: Kind::Bool, default: "1", label: "Show the game as modified", help: "The main menu marks the game as modified." },
    KeyDef { key: "autoresolve_hooks", group: Group::Sim, kind: Kind::Bool, default: "1", label: "Auto-resolve hooks", help: "Master switch for every auto-resolve hook (off also disables the duel hook)." },
    KeyDef { key: "duel_power_hook", group: Group::Sim, kind: Kind::Bool, default: "1", label: "Duel power hook", help: "Per-character duel bonus and stat formula in auto-resolve (190 Expanded \"Duel CEO\"). Off = those script calls are refused." },
    KeyDef { key: "prebattle_single_delegate", group: Group::Sim, kind: Kind::Bool, default: "1", label: "Multiplayer: one Delegate vote for all", help: "Before a battle, one human's Delegate vote counts for every human." },
    KeyDef { key: "postbattle_single_continue", group: Group::Sim, kind: Kind::Bool, default: "1", label: "Multiplayer: one Continue for all", help: "After a battle, one human's Continue clears the screen for every human (an unmade captive choice takes the default)." },
    KeyDef { key: "save_chunking", group: Group::Sim, kind: Kind::Bool, default: "1", label: "Save chunking", help: "Saved script values above the engine's 64 KiB limit are split into chunks. Off = vanilla (a large store resets on load)." },
    KeyDef { key: "marriage_inlaws", group: Group::Sim, kind: Kind::Bool, default: "1", label: "Relatives by marriage may marry", help: "Off = the engine rule: one marriage between two houses blocks every later one." },
    KeyDef { key: "marriage_blood_generations", group: Group::Sim, kind: Kind::Int { min: 0, max: 6 }, default: "0", label: "Blood-relative marriage ban (generations)", help: "With in-law marriage on: blood relatives sharing an ancestor within this many generations may not marry. 0 = only the engine's close-kin rule." },
    KeyDef { key: "horde_income", group: Group::Sim, kind: Kind::Bool, default: "1", label: "Horde income", help: "Adds faction- and force-scoped gdp_abs effects to the treasury." },
    KeyDef { key: "horde_income_category", group: Group::Sim, kind: Kind::Choice(&[("0", "Taxes"), ("1", "Mining (recommended)"), ("2", "Trade"), ("3", "Military force")]), default: "1", label: "Horde income line", help: "Which income line receives the horde income." },
    KeyDef { key: "followup_hooks", group: Group::Sim, kind: Kind::Bool, default: "1", label: "Mediate-peace button repair", help: "Repairs the MEDIATE PEACE diplomacy button." },
    KeyDef { key: "ai_recruit_hook", group: Group::Sim, kind: Kind::Bool, default: "1", label: "AI recruitment hook", help: "AI recruitment planner hook (AI scope for the recruit caches)." },
    KeyDef { key: "recruit_perm_cache", group: Group::Sim, kind: Kind::Choice(CACHE_MODES), default: "1", label: "Unit permission cache", help: "Reuses the unit-permission table: a large end-of-turn time saving." },
    KeyDef { key: "ai_recruit_cache", group: Group::Sim, kind: Kind::Choice(&[("0", "Off (recommended)"), ("1", "Verify only"), ("2", "Serve")]), default: "0", label: "AI recruit-list cache", help: "Diagnostic cache for AI recruit lists. Measured as not worth it: leave off." },
    KeyDef { key: "ui_recruit_cache_ms", group: Group::Local, kind: Kind::Int { min: 0, max: 30_000 }, default: "5000", label: "Recruit-list UI cache (ms)", help: "How long the recruit-list UI cache lives. 0 = off." },
    KeyDef { key: "file_probe_cache_ms", group: Group::Local, kind: Kind::Int { min: 0, max: 600_000 }, default: "10000", label: "Missing-file cache (ms)", help: "How long missing loose-file folders are remembered. 0 = off." },
    KeyDef { key: "diag_crash", group: Group::Local, kind: Kind::Choice(&[("0", "Off"), ("1", "On (se_crash.txt)"), ("2", "On + activity log")]), default: "1", label: "Crash reporter", help: "1 writes se_crash.txt next to the DLL when the game faults; 2 also logs every hook and native call to se_activity.txt (large)." },
    KeyDef { key: "diag_diplomacy", group: Group::Local, kind: Kind::Choice(&[("0", "Off"), ("1", "Counters, trace on request"), ("2", "Trace from injection")]), default: "0", label: "Diplomacy diagnostics", help: "Diplomacy counters and a trace in dip_trace.txt." },
];

pub fn def(key: &str) -> Option<&'static KeyDef> {
    KEYS.iter().find(|d| d.key == key)
}

/// Is `value` acceptable for `d`?
pub fn valid(d: &KeyDef, value: &str) -> bool {
    match d.kind {
        Kind::Bool => value == "0" || value == "1",
        Kind::Int { min, max } => value.parse::<i64>().map(|v| (min..=max).contains(&v)).unwrap_or(false),
        Kind::Choice(opts) => opts.iter().any(|(v, _)| *v == value),
        Kind::Text => !value.contains(['\n', '\r']),
    }
}

/// The settings someone chose: only values that differ from the defaults, plus any key this
/// table does not know (kept verbatim).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(transparent)]
pub struct SeConfig(pub BTreeMap<String, String>);

fn unquote(v: &str) -> &str {
    let v = v.trim();
    for q in ['"', '\''] {
        if v.len() >= 2 && v.starts_with(q) && v.ends_with(q) {
            return &v[1..v.len() - 1];
        }
    }
    v
}

impl SeConfig {
    pub fn parse(text: &str) -> SeConfig {
        let mut c = SeConfig::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let k = k.trim();
            if !k.is_empty() {
                c.set(k, unquote(v));
            }
        }
        c
    }

    /// Effective value: the chosen one if valid, else the default ("" for unknown keys).
    pub fn get(&self, key: &str) -> String {
        let chosen = self.0.get(key);
        match def(key) {
            Some(d) => chosen.filter(|v| valid(d, v)).cloned().unwrap_or_else(|| d.default.to_string()),
            None => chosen.cloned().unwrap_or_default(),
        }
    }

    /// Set a value; a known key's default is stored as "not set".
    pub fn set(&mut self, key: &str, value: &str) {
        let value = value.trim();
        match def(key) {
            Some(d) if value == d.default || (d.kind == Kind::Text && value.is_empty()) => {
                self.0.remove(key);
            }
            _ => {
                self.0.insert(key.to_string(), value.to_string());
            }
        }
    }

    pub fn is_default(&self, key: &str) -> bool {
        def(key).map(|d| self.get(key) == d.default).unwrap_or(false)
    }

    /// Keys the table does not know, with their values.
    pub fn unknown(&self) -> impl Iterator<Item = (&String, &String)> {
        self.0.iter().filter(|(k, _)| def(k).is_none())
    }

    /// The file the DLL reads. Every known key is written (see the module doc for why the
    /// simulation keys must be); invalid values fall back to the default.
    pub fn render(&self) -> String {
        let mut out = String::from("# script_extender.cfg - written by TK Mod Manager, read at injection\n");
        for (group, title) in [
            (Group::Menu, ""),
            (Group::Sim, "\n# --- simulation (multiplayer version lock: every player must match) ---\n"),
            (Group::Local, "\n# --- local ---\n"),
        ] {
            out.push_str(title);
            for d in KEYS.iter().filter(|d| d.group == group) {
                out.push_str(&format!("{}={}\n", d.key, self.get(d.key)));
            }
        }
        let mut other = self.unknown().peekable();
        if other.peek().is_some() {
            out.push_str("\n# --- other ---\n");
            for (k, v) in other {
                out.push_str(&format!("{k}={v}\n"));
            }
        }
        out
    }

    /// The simulation keys with their effective values (multiplayer comparison).
    pub fn sim_values(&self) -> BTreeMap<String, String> {
        KEYS.iter().filter(|d| d.group == Group::Sim).map(|d| (d.key.to_string(), self.get(d.key))).collect()
    }

    /// Adopt someone else's simulation values, keeping this config's local and menu keys.
    pub fn with_sim(&self, sim: &BTreeMap<String, String>) -> SeConfig {
        let mut c = self.clone();
        for (k, v) in sim {
            if def(k).map(|d| d.group == Group::Sim).unwrap_or(false) {
                c.set(k, v);
            }
        }
        c
    }
}

/// Simulation keys whose values differ: (key, mine, theirs). A key missing on their side
/// (older manager or DLL) counts as its default.
pub fn diff_sim(mine: &BTreeMap<String, String>, theirs: &BTreeMap<String, String>) -> Vec<(String, String, String)> {
    KEYS.iter()
        .filter(|d| d.group == Group::Sim)
        .filter_map(|d| {
            let a = mine.get(d.key).cloned().unwrap_or_else(|| d.default.to_string());
            let b = theirs.get(d.key).cloned().unwrap_or_else(|| d.default.to_string());
            (a != b).then(|| (d.key.to_string(), a, b))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thirteen_sim_keys() {
        assert_eq!(KEYS.iter().filter(|d| d.group == Group::Sim).count(), 13);
        for d in KEYS {
            assert!(valid(d, d.default), "{} default invalid", d.key);
        }
    }

    #[test]
    fn parse_keeps_unknown_and_drops_defaults() {
        let c = SeConfig::parse("# c\nbuild_number = \"v1.7 (190E)\"\nmarriage_inlaws=0\nrecruit_perm_cache=1\ndiag_ui_expr=1\njunk\n");
        assert_eq!(c.get("build_number"), "v1.7 (190E)");
        assert_eq!(c.get("marriage_inlaws"), "0");
        assert!(!c.0.contains_key("recruit_perm_cache"), "defaults are not stored");
        assert_eq!(c.unknown().collect::<Vec<_>>(), vec![(&"diag_ui_expr".to_string(), &"1".to_string())]);
        assert_eq!(SeConfig::parse(&c.render()), c);
    }

    #[test]
    fn render_writes_every_key_and_falls_back() {
        let mut c = SeConfig::default();
        c.set("horde_income_category", "9");
        c.set("marriage_blood_generations", "7");
        let text = c.render();
        for d in KEYS {
            assert!(text.contains(&format!("\n{}=", d.key)), "{} missing", d.key);
        }
        assert!(text.contains("horde_income_category=1\n"));
        assert!(text.contains("marriage_blood_generations=0\n"));
        assert!(text.contains("autoresolve_hooks=1\n") && text.contains("ai_recruit_cache=0\n"));
    }

    #[test]
    fn sim_diff_and_adopt() {
        let mut mine = SeConfig::default();
        mine.set("build_number", "mine");
        let mut theirs = SeConfig::default();
        theirs.set("marriage_inlaws", "0");
        theirs.set("diag_crash", "2");
        let d = diff_sim(&mine.sim_values(), &theirs.sim_values());
        assert_eq!(d, vec![("marriage_inlaws".to_string(), "1".to_string(), "0".to_string())]);
        // A partner export without the key means the default.
        assert!(diff_sim(&mine.sim_values(), &BTreeMap::new()).is_empty());
        let adopted = mine.with_sim(&theirs.sim_values());
        assert_eq!(adopted.get("marriage_inlaws"), "0");
        assert_eq!(adopted.get("build_number"), "mine");
        assert_eq!(adopted.get("diag_crash"), "1", "local keys stay mine");
    }
}
