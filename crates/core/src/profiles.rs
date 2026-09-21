//! Profiles: named, ordered enable lists keyed by mod identity. Stored in `profiles.json`.

use serde::{Deserialize, Serialize};

pub const SCHEMA: u32 = 2;

/// Key prefix of separator entries (group headers); they never resolve to a pack.
pub const SEPARATOR_PREFIX: &str = "sep:";

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProfileEntry {
    pub key: String,
    pub enabled: bool,
    /// Separator caption (only for `sep:` entries).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Separator's group is folded in the UI.
    #[serde(default, skip_serializing_if = "is_false")]
    pub collapsed: bool,
}

impl ProfileEntry {
    pub fn is_separator(&self) -> bool {
        self.key.starts_with(SEPARATOR_PREFIX)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    /// Load order = array order. Entries may reference packs that are no longer installed: they
    /// are kept so the load order survives a re-subscribe, hidden in the list, and purged only
    /// when the user asks (`profile_ops::remove_missing`).
    pub entries: Vec<ProfileEntry>,
    /// Inject the script extender after launch.
    #[serde(default)]
    pub dll: bool,
    /// Generate the skip-intro options pack on launch.
    #[serde(default)]
    pub skip_intro: bool,
    /// Unix seconds of the last launch with this profile (drives the "updated" badges).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_played: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProfilesDoc {
    pub schema: u32,
    pub active: String,
    pub profiles: Vec<Profile>,
}

impl Default for ProfilesDoc {
    fn default() -> Self {
        ProfilesDoc {
            schema: SCHEMA,
            active: "Default".into(),
            profiles: vec![Profile {
                name: "Default".into(),
                entries: vec![],
                dll: false,
                skip_intro: false,
                last_played: None,
            }],
        }
    }
}

