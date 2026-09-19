//! TK Mod Manager core: everything except the GUI. The Qt app (`crates/app`) drives these
//! modules; long-running calls (network, hashing, pack scans) are blocking and meant to run on
//! worker threads.

pub mod cli;
pub mod conflicts;
pub mod context;
pub mod dll;
pub mod empty_vp8;
pub mod fmt;
pub mod fingerprint;
pub mod groups;
pub mod hash;
pub mod history;
pub mod json_store;
pub mod launch;
pub mod logs;
pub mod meta;
pub mod modlist;
pub mod ops;
pub mod options_pack;
pub mod packs;
pub mod paths;
pub mod profile_ops;
pub mod profiles;
pub mod se_scan;
pub mod settings;
pub mod status;
pub mod steam_acf;
pub mod sync;
pub mod update;
pub mod winproc;
pub mod workshop;

pub use context::{AppContext, Ctx};
