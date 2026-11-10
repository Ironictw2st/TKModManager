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
use crate::t;
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
    /// One of the listed values (their text: `KeyDef::choice_text`).
    Choice(&'static [&'static str]),
    /// Free text on one line.
    Text,
}

/// What part of the game a setting is about: how the DLLs tab groups the features.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Battle,
    CampaignAi,
    Characters,
    Economy,
    Multiplayer,
    Stability,
    Performance,
    Interface,
    Diagnostics,
    MenuText,
}

impl Area {
    /// In the order the DLLs tab shows them.
    pub const ALL: [Area; 10] = [
        Area::Battle,
        Area::CampaignAi,
        Area::Characters,
        Area::Economy,
        Area::Multiplayer,
        Area::Stability,
        Area::Performance,
        Area::Interface,
        Area::Diagnostics,
        Area::MenuText,
    ];

    pub fn label(self) -> String {
        match self {
            Area::Battle => t!("se-area-battle"),
            Area::CampaignAi => t!("se-area-campaign-ai"),
            Area::Characters => t!("se-area-characters"),
            Area::Economy => t!("se-area-economy"),
            Area::Multiplayer => t!("se-area-multiplayer"),
            Area::Stability => t!("se-area-stability"),
            Area::Performance => t!("se-area-performance"),
            Area::Interface => t!("se-area-interface"),
            Area::Diagnostics => t!("se-area-diagnostics"),
            Area::MenuText => t!("se-area-menu-text"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct KeyDef {
    pub key: &'static str,
    pub group: Group,
    pub area: Area,
    pub kind: Kind,
    pub default: &'static str,
}

const CACHE_MODES: &[&str] = &["0", "1", "2"];
const B: Kind = Kind::Bool;

const fn k(key: &'static str, group: Group, area: Area, kind: Kind, default: &'static str) -> KeyDef {
    KeyDef { key, group, area, kind, default }
}

/// Every key the manager knows, in the order they are written. Defaults are the DLL's own
/// (TK-ScriptExtender `build.rs` KNOWN_KEYS, docs/SCRIPTING.md "The configuration file").
pub const KEYS: &[KeyDef] = &[
    k("build_number", Group::Menu, Area::MenuText, Kind::Text, "{game}"),
    k("build_number_short", Group::Menu, Area::MenuText, Kind::Text, "{game}"),
    k("build_modified", Group::Menu, Area::MenuText, B, "1"),
    k("autoresolve_hooks", Group::Sim, Area::Battle, B, "1"),
    k("duel_power_hook", Group::Sim, Area::Battle, B, "1"),
    k("prebattle_single_delegate", Group::Sim, Area::Multiplayer, B, "1"),
    k("postbattle_single_continue", Group::Sim, Area::Multiplayer, B, "1"),
    k("save_chunking", Group::Sim, Area::Stability, B, "1"),
    k("marriage_inlaws", Group::Sim, Area::Characters, B, "1"),
    k("marriage_blood_generations", Group::Sim, Area::Characters, Kind::Int { min: 0, max: 6 }, "0"),
    k("horde_income", Group::Sim, Area::Economy, B, "1"),
    k("horde_income_category", Group::Sim, Area::Economy, Kind::Choice(&["0", "1", "2", "3"]), "1"),
    k("followup_hooks", Group::Sim, Area::Characters, B, "1"),
    k("ai_recruit_hook", Group::Sim, Area::CampaignAi, B, "1"),
    k("recruit_perm_cache", Group::Sim, Area::Performance, Kind::Choice(CACHE_MODES), "1"),
    k("ai_recruit_cache", Group::Sim, Area::Performance, Kind::Choice(&["0", "1", "2"]), "0"),
    k("naval_to_land", Group::Sim, Area::Battle, B, "1"),
    k("followup_respond_hook", Group::Sim, Area::Characters, B, "1"),
    k("engine_crash_guards", Group::Sim, Area::Stability, B, "1"),
    k("cai_null_guard", Group::Sim, Area::Stability, B, "1"),
    k("siege_force_assault", Group::Sim, Area::CampaignAi, B, "1"),
    k("siege_odds_gate", Group::Sim, Area::CampaignAi, B, "0"),
    k("siege_odds_bands", Group::Sim, Area::CampaignAi, Kind::Text, "1.5:25,3:10,6:2.5"),
    k("ai_skills", Group::Sim, Area::CampaignAi, B, "1"),
    k("ai_ceos", Group::Sim, Area::CampaignAi, B, "1"),
    k("ai_court", Group::Sim, Area::CampaignAi, B, "1"),
    k("ai_build", Group::Sim, Area::CampaignAi, B, "1"),
    k("ui_recruit_cache_ms", Group::Local, Area::Performance, Kind::Int { min: 0, max: 30_000 }, "5000"),
    k("file_probe_cache_ms", Group::Local, Area::Performance, Kind::Int { min: 0, max: 600_000 }, "10000"),
    k("battle_xform_fast", Group::Local, Area::Performance, B, "1"),
    k("ceo_ui_refresh", Group::Local, Area::Interface, B, "1"),
    k("audio", Group::Local, Area::Interface, B, "1"),
    k("diag_crash", Group::Local, Area::Diagnostics, Kind::Choice(&["0", "1", "2"]), "1"),
    k("diag_diplomacy", Group::Local, Area::Diagnostics, Kind::Choice(&["0", "1", "2"]), "0"),
    k("ui_dev_loader", Group::Local, Area::Diagnostics, B, "0"),
];

impl KeyDef {
    /// The setting's name for people.
    pub fn label(&self) -> String {
        match self.key {
            "build_number" => t!("se-key-build-number"),
            "build_number_short" => t!("se-key-build-number-short"),
            "build_modified" => t!("se-key-build-modified"),
            "autoresolve_hooks" => t!("se-key-autoresolve-hooks"),
            "duel_power_hook" => t!("se-key-duel-power-hook"),
            "prebattle_single_delegate" => t!("se-key-prebattle-single-delegate"),
            "postbattle_single_continue" => t!("se-key-postbattle-single-continue"),
            "save_chunking" => t!("se-key-save-chunking"),
            "marriage_inlaws" => t!("se-key-marriage-inlaws"),
            "marriage_blood_generations" => t!("se-key-marriage-blood-generations"),
            "horde_income" => t!("se-key-horde-income"),
            "horde_income_category" => t!("se-key-horde-income-category"),
            "followup_hooks" => t!("se-key-followup-hooks"),
            "ai_recruit_hook" => t!("se-key-ai-recruit-hook"),
            "recruit_perm_cache" => t!("se-key-recruit-perm-cache"),
            "ai_recruit_cache" => t!("se-key-ai-recruit-cache"),
            "ui_recruit_cache_ms" => t!("se-key-ui-recruit-cache-ms"),
            "file_probe_cache_ms" => t!("se-key-file-probe-cache-ms"),
            "diag_crash" => t!("se-key-diag-crash"),
            "diag_diplomacy" => t!("se-key-diag-diplomacy"),
            "naval_to_land" => t!("se-key-naval-to-land"),
            "followup_respond_hook" => t!("se-key-followup-respond-hook"),
            "engine_crash_guards" => t!("se-key-engine-crash-guards"),
            "cai_null_guard" => t!("se-key-cai-null-guard"),
            "siege_force_assault" => t!("se-key-siege-force-assault"),
            "siege_odds_gate" => t!("se-key-siege-odds-gate"),
            "siege_odds_bands" => t!("se-key-siege-odds-bands"),
            "ai_skills" => t!("se-key-ai-skills"),
            "ai_ceos" => t!("se-key-ai-ceos"),
            "ai_court" => t!("se-key-ai-court"),
            "ai_build" => t!("se-key-ai-build"),
            "battle_xform_fast" => t!("se-key-battle-xform-fast"),
            "ceo_ui_refresh" => t!("se-key-ceo-ui-refresh"),
            "audio" => t!("se-key-audio"),
            "ui_dev_loader" => t!("se-key-ui-dev-loader"),
            other => other.to_string(),
        }
    }

    /// One or two sentences on what the setting does.
    pub fn help(&self) -> String {
        match self.key {
            "build_number" => t!("se-key-build-number-help"),
            "build_number_short" => t!("se-key-build-number-short-help"),
            "build_modified" => t!("se-key-build-modified-help"),
            "autoresolve_hooks" => t!("se-key-autoresolve-hooks-help"),
            "duel_power_hook" => t!("se-key-duel-power-hook-help"),
            "prebattle_single_delegate" => t!("se-key-prebattle-single-delegate-help"),
            "postbattle_single_continue" => t!("se-key-postbattle-single-continue-help"),
            "save_chunking" => t!("se-key-save-chunking-help"),
            "marriage_inlaws" => t!("se-key-marriage-inlaws-help"),
            "marriage_blood_generations" => t!("se-key-marriage-blood-generations-help"),
            "horde_income" => t!("se-key-horde-income-help"),
            "horde_income_category" => t!("se-key-horde-income-category-help"),
            "followup_hooks" => t!("se-key-followup-hooks-help"),
            "ai_recruit_hook" => t!("se-key-ai-recruit-hook-help"),
            "recruit_perm_cache" => t!("se-key-recruit-perm-cache-help"),
            "ai_recruit_cache" => t!("se-key-ai-recruit-cache-help"),
            "ui_recruit_cache_ms" => t!("se-key-ui-recruit-cache-ms-help"),
            "file_probe_cache_ms" => t!("se-key-file-probe-cache-ms-help"),
            "diag_crash" => t!("se-key-diag-crash-help"),
            "diag_diplomacy" => t!("se-key-diag-diplomacy-help"),
            "naval_to_land" => t!("se-key-naval-to-land-help"),
            "followup_respond_hook" => t!("se-key-followup-respond-hook-help"),
            "engine_crash_guards" => t!("se-key-engine-crash-guards-help"),
            "cai_null_guard" => t!("se-key-cai-null-guard-help"),
            "siege_force_assault" => t!("se-key-siege-force-assault-help"),
            "siege_odds_gate" => t!("se-key-siege-odds-gate-help"),
            "siege_odds_bands" => t!("se-key-siege-odds-bands-help"),
            "ai_skills" => t!("se-key-ai-skills-help"),
            "ai_ceos" => t!("se-key-ai-ceos-help"),
            "ai_court" => t!("se-key-ai-court-help"),
            "ai_build" => t!("se-key-ai-build-help"),
            "battle_xform_fast" => t!("se-key-battle-xform-fast-help"),
            "ceo_ui_refresh" => t!("se-key-ceo-ui-refresh-help"),
            "audio" => t!("se-key-audio-help"),
            "ui_dev_loader" => t!("se-key-ui-dev-loader-help"),
            _ => String::new(),
        }
    }

    /// The text for one value of a `Kind::Choice` key (the value itself when unknown).
    pub fn choice_text(&self, value: &str) -> String {
        match (self.key, value) {
            ("horde_income_category", "0") => t!("se-key-horde-income-category-0"),
            ("horde_income_category", "1") => t!("se-key-horde-income-category-1"),
            ("horde_income_category", "2") => t!("se-key-horde-income-category-2"),
            ("horde_income_category", "3") => t!("se-key-horde-income-category-3"),
            ("recruit_perm_cache", "0") => t!("se-key-recruit-perm-cache-0"),
            ("recruit_perm_cache", "1") => t!("se-key-recruit-perm-cache-1"),
            ("recruit_perm_cache", "2") => t!("se-key-recruit-perm-cache-2"),
            ("ai_recruit_cache", "0") => t!("se-key-ai-recruit-cache-0"),
            ("ai_recruit_cache", "1") => t!("se-key-ai-recruit-cache-1"),
            ("ai_recruit_cache", "2") => t!("se-key-ai-recruit-cache-2"),
            ("diag_crash", "0") => t!("se-key-diag-crash-0"),
            ("diag_crash", "1") => t!("se-key-diag-crash-1"),
            ("diag_crash", "2") => t!("se-key-diag-crash-2"),
            ("diag_diplomacy", "0") => t!("se-key-diag-diplomacy-0"),
            ("diag_diplomacy", "1") => t!("se-key-diag-diplomacy-1"),
            ("diag_diplomacy", "2") => t!("se-key-diag-diplomacy-2"),
            _ => value.to_string(),
        }
    }
}

pub fn def(key: &str) -> Option<&'static KeyDef> {
    KEYS.iter().find(|d| d.key == key)
}

/// Is `value` acceptable for `d`?
pub fn valid(d: &KeyDef, value: &str) -> bool {
    match d.kind {
        Kind::Bool => value == "0" || value == "1",
        Kind::Int { min, max } => value.parse::<i64>().map(|v| (min..=max).contains(&v)).unwrap_or(false),
        Kind::Choice(opts) => opts.contains(&value),
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

    /// The DLL's KNOWN_KEYS (TK-ScriptExtender 0.47.0-beta.4, build.rs) and the keys its
    /// `sync_tag()` hashes. Update both when the DLL gains a setting.
    const DLL_KEYS: &[&str] = &[
        "build_number", "build_number_short", "build_modified",
        "autoresolve_hooks", "horde_income", "horde_income_category", "ai_recruit_hook", "followup_hooks",
        "marriage_inlaws", "marriage_blood_generations",
        "ui_recruit_cache_ms", "ai_recruit_cache", "recruit_perm_cache", "file_probe_cache_ms",
        "diag_diplomacy", "diag_crash", "save_chunking", "duel_power_hook", "prebattle_single_delegate", "postbattle_single_continue",
        "naval_to_land", "followup_respond_hook", "ceo_ui_refresh", "audio", "ui_dev_loader", "engine_crash_guards",
        "cai_null_guard", "battle_xform_fast", "siege_odds_gate", "siege_odds_bands", "siege_force_assault",
        "ai_skills", "ai_ceos", "ai_court", "ai_build",
    ];
    const DLL_SYNC_KEYS: &[&str] = &[
        "autoresolve_hooks", "ai_recruit_cache", "recruit_perm_cache", "horde_income", "horde_income_category",
        "ai_recruit_hook", "followup_hooks", "marriage_inlaws", "save_chunking", "duel_power_hook", "prebattle_single_delegate",
        "postbattle_single_continue", "naval_to_land", "followup_respond_hook", "engine_crash_guards", "cai_null_guard",
        "siege_force_assault", "siege_odds_gate", "siege_odds_bands", "ai_skills", "ai_ceos", "ai_court", "ai_build",
        "marriage_blood_generations",
    ];

    #[test]
    fn table_matches_the_dll() {
        let mut ours: Vec<&str> = KEYS.iter().map(|d| d.key).collect();
        let mut theirs = DLL_KEYS.to_vec();
        ours.sort();
        theirs.sort();
        assert_eq!(ours, theirs);
        let mut sim: Vec<&str> = KEYS.iter().filter(|d| d.group == Group::Sim).map(|d| d.key).collect();
        let mut sync = DLL_SYNC_KEYS.to_vec();
        sim.sort();
        sync.sort();
        assert_eq!(sim, sync, "Group::Sim must be exactly the keys in the DLL's version lock");
        for d in KEYS {
            assert!(valid(d, d.default), "{} default invalid", d.key);
            assert!(Area::ALL.contains(&d.area));
            assert_ne!(d.label(), d.key, "{} has no label", d.key);
            assert!(!d.help().is_empty(), "{} has no help", d.key);
        }
        for a in Area::ALL {
            assert!(KEYS.iter().any(|d| d.area == a), "{a:?} is empty");
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
