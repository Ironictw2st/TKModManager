//! Profiles: named, ordered enable lists keyed by mod identity. Stored in `profiles.json`.

use serde::{Deserialize, Serialize};

pub const SCHEMA: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProfileEntry {
    pub key: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    /// Load order = array order. Entries may reference packs that are no longer installed.
    pub entries: Vec<ProfileEntry>,
    /// Inject the script extender after launch.
    #[serde(default)]
    pub dll: bool,
    /// Generate the skip-intro options pack on launch.
    #[serde(default)]
    pub skip_intro: bool,
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
            }],
        }
    }
}

