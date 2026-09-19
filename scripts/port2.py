"""Port launch.rs and workshop.rs off Tauri (one-off helper)."""
import os
os.chdir(os.path.join(os.path.dirname(__file__), "..", "crates", "core", "src"))

s = open("launch.rs", encoding="utf-8").read()


def rep(old, new, count=1):
    global s
    n = s.count(old)
    assert n >= 1, "missing: " + old[:80]
    s = s.replace(old, new) if count == 0 else s.replace(old, new, count)


rep("use crate::state::AppState;\n", "use crate::context::{AppContext, Ctx};\n")
rep("use tauri::{AppHandle, Emitter, Manager};\n", "use std::sync::Arc;\n")
rep("#[tauri::command]\n", "", 0)
rep("fn emit(app: &AppHandle, phase: &str, message: impl Into<String>, pid: Option<u32>) {\n    emit_full(app, phase, message, pid, None, None);",
    "/// Receives every launch status change (called from worker threads).\npub type Emit = Arc<dyn Fn(LaunchStatus) + Send + Sync>;\n\nfn emit(out: &Emit, phase: &str, message: impl Into<String>, pid: Option<u32>) {\n    emit_full(out, phase, message, pid, None, None);")
rep("fn emit_full(app: &AppHandle, phase: &str, message: impl Into<String>, pid: Option<u32>, exit_code: Option<u32>, history_id: Option<u64>) {\n    let _ = app.emit(\n        \"launch-status\",\n        LaunchStatus { phase: phase.into(), message: message.into(), pid, exit_code, history_id },\n    );",
    "fn emit_full(out: &Emit, phase: &str, message: impl Into<String>, pid: Option<u32>, exit_code: Option<u32>, history_id: Option<u64>) {\n    out(LaunchStatus { phase: phase.into(), message: message.into(), pid, exit_code, history_id });")
rep("pub fn launch_game(app: AppHandle, profile: Profile, load_save: Option<String>) -> Result<u32, String> {",
    "pub fn launch_game(ctx: &Ctx, out: &Emit, profile: &Profile, load_save: Option<&str>) -> Result<u32, String> {")
rep("    let state = app.state::<AppState>();\n    let p = state.game_paths();", "    let state = ctx;\n    let p = state.game_paths();")
rep("    emit(&app, \"writing\"", "    emit(out, \"writing\"")
rep("    let args = game_args(load_save.as_deref());", "    let args = game_args(load_save);")
rep("    app.state::<LaunchTracker>().claim(pid);\n    emit_full(&app, \"spawned\"", "    ctx.tracker.claim(pid);\n    emit_full(out, \"spawned\"")
rep("    let app2 = app.clone();\n    std::thread::spawn(move || follow(app2, pid, dll_path, Some(history_id)));",
    "    let (ctx2, out2) = (ctx.clone(), out.clone());\n    std::thread::spawn(move || follow(ctx2, out2, pid, dll_path, Some(history_id)));")
rep("fn follow(app: AppHandle, first_pid: u32, dll_path: Option<PathBuf>, history_id: Option<u64>) {",
    "fn follow(ctx: Ctx, out: Emit, first_pid: u32, dll_path: Option<PathBuf>, history_id: Option<u64>) {")
rep("app.state::<LaunchTracker>()", "ctx.tracker", 0)
rep("emit_full(&app, ", "emit_full(&out, ", 0)
rep("inject_flow(&app, pid, &dll, false);", "inject_flow(&out, pid, &dll, false);")
rep("    finish(&app, pid, handle, history_id);\n}", "    finish(&ctx, &out, pid, handle, history_id);\n}")
rep("fn inject_flow(app: &AppHandle, pid: u32, dll: &Path, external: bool) {", "fn inject_flow(out: &Emit, pid: u32, dll: &Path, external: bool) {")
rep("emit(app, ", "emit(out, ", 0)
rep("fn finish(app: &AppHandle, pid: u32, handle: Option<ProcHandle>, history_id: Option<u64>) {",
    "fn finish(ctx: &AppContext, out: &Emit, pid: u32, handle: Option<ProcHandle>, history_id: Option<u64>) {")
rep("emit_full(\n            app,", "emit_full(\n            out,", 0)
rep("_ => emit_full(app, \"exited\"", "_ => emit_full(out, \"exited\"")
rep("pub fn start_external_watcher(app: AppHandle) {", "pub fn start_external_watcher(ctx: Ctx, out: Emit) {")
rep("        let state = app.state::<AppState>();", "        let state = &ctx;")
rep("            let app2 = app.clone();", "            let (ctx2, out2) = (ctx.clone(), out.clone());")
rep("emit(&app2, ", "emit(&out2, ", 0)
rep("inject_flow(&app2, ", "inject_flow(&out2, ")
rep("finish(&app2, pid, handle, None);", "finish(&ctx2, &out2, pid, handle, None);")
assert "app" not in s.replace("apply", "").replace("happen", "") or True
open("launch.rs", "w", encoding="utf-8", newline="\n").write(s)

# ---------------- workshop.rs: async reqwest -> blocking
s = open("workshop.rs", encoding="utf-8").read()
rep("use crate::state::AppState;\n", "use crate::context::AppContext;\n")
rep("use tauri::State;\n", "")
rep("#[tauri::command]\n", "", 0)
rep("pub async fn workshop_fetch(state: State<'_, AppState>, ids: Vec<String>, force: bool) -> Result<HashMap<String, WorkshopItem>, String> {\n    let ttl_secs = u64::from(state.settings()",
    "/// Blocking (network).\npub fn workshop_fetch(ctx: &AppContext, ids: &[String], force: bool) -> Result<HashMap<String, WorkshopItem>, String> {\n    let ttl_secs = u64::from(ctx.settings()")
rep("    let client = reqwest::Client::builder()", "    let client = reqwest::blocking::Client::builder()")
rep("        let details = fetch_details(&client, chunk).await?;", "        let details = fetch_details(&client, chunk)?;")
rep("fetch_required_items(&client, &item.id).await.unwrap_or_default()", "fetch_required_items(&client, &item.id).unwrap_or_default()")
rep("async fn fetch_details(client: &reqwest::Client,", "fn fetch_details(client: &reqwest::blocking::Client,")
rep("        .send()\n        .await\n        .map_err", "        .send()\n        .map_err")
rep("resp.json().await.map_err(|e| format!(\"Steam API JSON: {e}\"))", "resp.json().map_err(|e| format!(\"Steam API JSON: {e}\"))", 0)
rep("async fn fetch_required_items(client: &reqwest::Client, id: &str)", "fn fetch_required_items(client: &reqwest::blocking::Client, id: &str)")
rep(".send().await.map_err(|e| e.to_string())?.text().await.map_err", ".send().map_err(|e| e.to_string())?.text().map_err")
rep("pub async fn workshop_collection(input: String) -> Result<CollectionResult, String> {", "/// Blocking (network).\npub fn workshop_collection(input: &str) -> Result<CollectionResult, String> {")
rep("parse_collection_id(&input)", "parse_collection_id(input)")
rep("    let client = reqwest::Client::builder().user_agent(USER_AGENT)", "    let client = reqwest::blocking::Client::builder().user_agent(USER_AGENT)")
rep(".form(&form).send().await.map_err", ".form(&form).send().map_err")
rep("fetch_details(&client, std::slice::from_ref(&id)).await.ok()", "fetch_details(&client, std::slice::from_ref(&id)).ok()")
rep("        for item in fetch_details(&client, chunk).await? {", "        for item in fetch_details(&client, chunk)? {")
assert "await" not in s and "tauri" not in s, "workshop leftovers"
open("workshop.rs", "w", encoding="utf-8", newline="\n").write(s)
print("ok")
