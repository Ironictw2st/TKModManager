"""Strip Tauri from the moved backend files (one-off port helper)."""
import os, re
os.chdir(os.path.join(os.path.dirname(__file__), "..", "crates", "core", "src"))


def rw(p, pairs):
    s = open(p, encoding="utf-8").read()
    for old, new in pairs:
        assert old in s, p + " missing: " + old[:90]
        s = s.replace(old, new, 1)
    open(p, "w", encoding="utf-8", newline="\n").write(s)


# ---------------- ops.rs (was commands.rs)
s = open("ops.rs", encoding="utf-8").read()
head_end = s.index("/// One line of an import")
new_head = '''//! Profile / list-file operations shared by the UI: loading and saving the JSON documents,
//! importing CA's `used_mods.txt`, and building the `tkmm_mods.txt` text for a profile.

use crate::context::{self, AppContext};
use crate::json_store;
use crate::meta::MetaDoc;
use crate::modlist::{self, ListInput, ListMod, ParsedList};
use crate::packs::{self, ModEntry, ModSource, PackType};
use crate::paths;
use crate::profiles::{Profile, ProfilesDoc};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn load_profiles() -> Result<ProfilesDoc, String> {
    json_store::load::<ProfilesDoc>(&context::profiles_path())
}

pub fn save_profiles(doc: &ProfilesDoc) -> Result<(), String> {
    json_store::save(&context::profiles_path(), doc)
}

pub fn load_meta() -> Result<MetaDoc, String> {
    json_store::load::<MetaDoc>(&context::meta_path())
}

pub fn save_meta(doc: &MetaDoc) -> Result<(), String> {
    json_store::save(&context::meta_path(), doc)
}

/// Parse a CA-style list file (`used_mods.txt`) into its directives.
pub fn read_mod_list_file(path: &str) -> Result<ParsedList, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(modlist::parse(&text))
}

'''
s = new_head + s[head_end:]
s = s.replace('''#[tauri::command]
pub fn import_mod_list(state: State<AppState>, path: String) -> Result<Vec<ImportedEntry>, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let parsed = modlist::parse(&text);
    let scan = state.scan();''', '''pub fn import_mod_list(ctx: &AppContext, path: &str) -> Result<Vec<ImportedEntry>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let parsed = modlist::parse(&text);
    let scan = ctx.scan();''')
s = s.replace('''#[tauri::command]
pub fn preview_mod_list(state: State<AppState>, profile: Profile) -> String {
    let scan = state.scan();
    modlist::build(&list_input_for(&profile, &scan))''', '''pub fn preview_mod_list(ctx: &AppContext, profile: &Profile) -> String {
    let scan = ctx.scan();
    modlist::build(&list_input_for(profile, &scan))''')
assert "tauri" not in s, "ops.rs still mentions tauri"
open("ops.rs", "w", encoding="utf-8", newline="\n").write(s)

# ---------------- meta.rs: manual override removed (unknown fields in old files are ignored)
rw("meta.rs", [('''    /// Manual script-extender requirement: None = automatic (marker / Lua scan).
    #[serde(default, rename = "seOverride", skip_serializing_if = "Option::is_none")]
    pub se_override: Option<bool>,
''', "")])

# ---------------- history.rs
rw("history.rs", [("#[tauri::command]\npub fn launch_history()", "pub fn launch_history()"),
                  ("#[tauri::command]\npub fn crash_report(", "pub fn crash_report(")])

# ---------------- logs.rs
rw("logs.rs", [
    ("use crate::state::AppState;\n", "use crate::context::AppContext;\n"),
    ("use tauri::State;\n", ""),
    ("#[tauri::command]\npub fn log_sources(state: State<AppState>) -> Vec<LogSource> {\n    let mut out = Vec::new();\n    let p = state.game_paths();",
     "pub fn log_sources(ctx: &AppContext) -> Vec<LogSource> {\n    let mut out = Vec::new();\n    let p = ctx.game_paths();"),
    ("#[tauri::command]\npub fn log_tail(state: State<AppState>, path: String, max_bytes: Option<u64>) -> Result<String, String> {\n    let p = state.game_paths();",
     "pub fn log_tail(ctx: &AppContext, path: &str, max_bytes: Option<u64>) -> Result<String, String> {\n    let p = ctx.game_paths();"),
])

# ---------------- conflicts.rs
rw("conflicts.rs", [
    ("use tauri::State;\n", ""),
    ("#[tauri::command]\npub fn conflicts_for(state: State<crate::state::AppState>, cache: State<ConflictCache>, keys: Vec<String>) -> ConflictReport {\n    let scan = state.scan();",
     "pub fn conflicts_for(ctx: &crate::context::AppContext, keys: &[String]) -> ConflictReport {\n    let cache = &ctx.conflicts;\n    let scan = ctx.scan();"),
    ("    report(&cache, &mods)", "    report(cache, &mods)"),
])

# ---------------- cli.rs
s = open("cli.rs", encoding="utf-8").read()
start = s.index("/// Args of this process, handed to the frontend once")
end = s.index("/// Create `Desktop\\TK3K")
s = s[:start] + s[end:]
s = s.replace("use std::sync::Mutex;\nuse tauri::State;\n", "")
s = s.replace("#[tauri::command]\npub fn create_profile_shortcut(profile: String)", "pub fn create_profile_shortcut(profile: &str)")
assert "tauri" not in s, "cli.rs"
open("cli.rs", "w", encoding="utf-8", newline="\n").write(s)

# ---------------- se_scan.rs
rw("se_scan.rs", [
    ("use crate::state::AppState;\n", "use crate::context::AppContext;\n"),
    ("use tauri::{AppHandle, Manager};\n", ""),
    ('''#[tauri::command]
pub async fn se_requirements(app: AppHandle, keys: Vec<String>) -> Result<HashMap<String, SeInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scan = app.state::<AppState>().scan();''', '''pub fn se_requirements(ctx: &AppContext, keys: &[String]) -> HashMap<String, SeInfo> {
    {
        let scan = ctx.scan();'''),
    ('''        if dirty {
            let _ = json_store::save(&cache_path(), &cache);
        }
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}''', '''        if dirty {
            let _ = json_store::save(&cache_path(), &cache);
        }
        out
    }
}'''),
])

# ---------------- hash.rs
rw("hash.rs", [
    ("use crate::state::AppState;\n", "use crate::context::AppContext;\n"),
    ("use tauri::{AppHandle, Emitter, Manager};\n", ""),
    ('''#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    file: String,
    done_bytes: u64,
    total_bytes: u64,
}''', '''#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub file: String,
    pub done_bytes: u64,
    pub total_bytes: u64,
}'''),
    ('''#[tauri::command]
pub async fn hash_packs(app: AppHandle, keys: Vec<String>) -> Result<Vec<PackHash>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let scan = state.scan();''', '''/// Blocking; call from a worker thread. `progress` is called every ~32 MB.
pub fn hash_packs(ctx: &AppContext, keys: &[String], progress: &dyn Fn(Progress)) -> Result<Vec<PackHash>, String> {
    {
        let scan = ctx.scan();'''),
    ('''                            let _ = app.emit("hash-progress", Progress { file: m.file.clone(), done_bytes: done, total_bytes: total });''',
     '''                            progress(Progress { file: m.file.clone(), done_bytes: done, total_bytes: total });'''),
    ('''        let _ = json_store::save(&cache_path(), &cache);
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}''', '''        let _ = json_store::save(&cache_path(), &cache);
        Ok(out)
    }
}'''),
])
print("ok")
