//! Per-mod user annotations (tags, notes, hidden), keyed by mod identity. `mods.meta.json`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ModMeta {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub hidden: bool,
    /// Manual script-extender requirement: None = automatic (marker / Lua scan).
    #[serde(default, rename = "seOverride", skip_serializing_if = "Option::is_none")]
    pub se_override: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct MetaDoc {
    #[serde(default)]
    pub mods: BTreeMap<String, ModMeta>,
}
