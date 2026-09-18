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
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct MetaDoc {
    #[serde(default)]
    pub mods: BTreeMap<String, ModMeta>,
}
