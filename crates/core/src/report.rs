//! "Create mod report": a plain-text file a player can send to a mod author when the game
//! crashes. It lists every enabled pack in load order with where it came from (Workshop id and
//! link, Nexus file, data folder), which version is installed against the newest one Steam or
//! Nexus knows of, and the file on disk; then the things worth a second look (pending updates,
//! Workshop items Steam no longer shows, movie packs, duplicate names, missing packs), the exact
//! list file the game is given, and every other pack that is installed but not enabled.

use crate::context::AppContext;
use crate::fmt::{format_bytes, ymd};
use crate::packs::{ModEntry, ModSource, PackType};
use crate::profiles::Profile;
use crate::se_scan::SeInfo;
use crate::workshop::WorkshopItem;
use crate::{dll, history, modlist, ops, paths, se_scan, workshop};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Everything the report is rendered from (gathered by `collect`, kept separate for tests).
pub struct ReportData<'a> {
    pub now: u64,
    pub profile: &'a Profile,
    pub scan: &'a [ModEntry],
    /// Steam's details per Workshop id. An id missing here means Steam returned nothing for it.
    pub workshop: &'a HashMap<String, WorkshopItem>,
    /// Whether `workshop` was fetched from Steam just now (else: the manager's cache).
    pub workshop_live: Result<(), String>,
    pub se: &'a HashMap<String, SeInfo>,
    pub game: &'a paths::GamePaths,
    /// Script-extender DLL a launch would inject, if any.
    pub dll: Option<String>,
    pub launches: &'a [history::LaunchRecord],
    /// Extra sections read from this PC (crash files, Windows error events, log ends), as
    /// (heading, body). Appended as they are.
    pub extras: Vec<(String, String)>,
}

/// "2026-09-19 21:04 UTC"; "unknown" for 0.
fn when(unix: u64) -> String {
    if unix == 0 {
        return "unknown".into();
    }
    let (y, m, d) = ymd(unix);
    let s = unix % 86_400;
    format!("{y}-{m:02}-{d:02} {:02}:{:02} UTC", s / 3600, s % 3600 / 60)
}

fn source_name(m: &ModEntry) -> &'static str {
    match m.source {
        ModSource::Workshop => "Steam Workshop",
        ModSource::Data => "game data folder",
        ModSource::Folder => "extra folder",
        ModSource::Nexus => "Nexus Mods",
    }
}

fn type_name(t: PackType) -> &'static str {
    match t {
        PackType::Mod => "mod",
        PackType::Movie => "movie (loads automatically)",
        PackType::Boot => "boot",
        PackType::Release => "release",
        PackType::Patch => "patch",
        PackType::Unknown => "unknown",
    }
}

fn title_of(m: &ModEntry, ws: &HashMap<String, WorkshopItem>) -> String {
    if let Some(n) = m.nexus.as_ref().filter(|n| !n.mod_name.is_empty()) {
        return n.mod_name.clone();
    }
    m.workshop_id
        .as_ref()
        .and_then(|id| ws.get(id))
        .map(|w| w.title.clone())
        .filter(|t| !t.is_empty())
        .unwrap_or_default()
}

/// Newest version date the manager knows of for a Workshop item: Steam's API, else Steam's own
/// download manifest.
fn ws_latest(m: &ModEntry, ws: &HashMap<String, WorkshopItem>) -> u64 {
    let api = m.workshop_id.as_ref().and_then(|id| ws.get(id)).filter(|w| !w.from_launcher_cache).map(|w| w.time_updated).unwrap_or(0);
    api.max(m.latest_updated.unwrap_or(0))
}

fn ws_pending(m: &ModEntry, ws: &HashMap<String, WorkshopItem>) -> bool {
    m.source == ModSource::Workshop && m.installed_updated.map(|i| ws_latest(m, ws) > i).unwrap_or(false)
}

/// Steam returned nothing for the id although we asked it just now: the item is hidden,
/// private or removed, and nobody else can download that version.
fn ws_gone(m: &ModEntry, d: &ReportData) -> bool {
    m.source == ModSource::Workshop
        && d.workshop_live.is_ok()
        && m.workshop_id.as_ref().map(|id| d.workshop.get(id).map(|w| w.from_launcher_cache).unwrap_or(true)).unwrap_or(false)
}

fn se_line(info: &SeInfo) -> Option<String> {
    if !info.required {
        return None;
    }
    let range = match (&info.min_version, &info.max_version) {
        (Some(a), Some(b)) => format!(" {a} to {b}"),
        (Some(a), None) => format!(" {a} or newer"),
        (None, Some(b)) => format!(" up to {b}"),
        (None, None) => String::new(),
    };
    Some(format!("needs the script extender{range}"))
}

fn pack_block(out: &mut String, n: usize, m: &ModEntry, d: &ReportData) {
    let _ = writeln!(out, "{n:>3}. {}", m.file);
    let title = title_of(m, d.workshop);
    if !title.is_empty() {
        let _ = writeln!(out, "     Title:    {title}");
    }
    match m.source {
        ModSource::Workshop => {
            let id = m.workshop_id.clone().unwrap_or_default();
            let _ = writeln!(out, "     Source:   Steam Workshop {id}  https://steamcommunity.com/sharedfiles/filedetails/?id={id}");
            let installed = m.installed_updated.unwrap_or(0);
            let latest = ws_latest(m, d.workshop);
            let state = if ws_gone(m, d) {
                "Steam has no public page for this item any more (hidden, private or removed)"
            } else if ws_pending(m, d.workshop) {
                "UPDATE PENDING: Steam has a newer version than the one installed"
            } else if installed == 0 {
                "installed version unknown"
            } else {
                "up to date"
            };
            let _ = writeln!(out, "     Version:  installed {} · newest on Steam {} · {state}", when(installed), if latest == 0 { "unknown".into() } else { when(latest) });
        }
        ModSource::Nexus => {
            if let Some(nx) = &m.nexus {
                let id = nx.mod_id.map(|i| format!(" mod {i}")).unwrap_or_default();
                let _ = writeln!(out, "     Source:   Nexus Mods{id} ({})", if nx.author.is_empty() { "author unknown" } else { &nx.author });
                let f = &nx.active;
                let ver = if f.version.is_empty() { String::new() } else { format!(" v{}", f.version) };
                let _ = writeln!(out, "     Version:  {}{ver} · uploaded {} · installed {}", f.name, when(f.uploaded), when(f.installed));
                if let Some(newer) = &nx.newer {
                    let _ = writeln!(out, "     Newer:    {} v{} uploaded {} is on Nexus", newer.name, newer.version, when(newer.uploaded));
                }
            } else {
                let _ = writeln!(out, "     Source:   Nexus Mods");
            }
        }
        ModSource::Data => {
            let _ = writeln!(out, "     Source:   game data folder (copied in by hand or a local build, not from the Workshop)");
        }
        ModSource::Folder => {
            let _ = writeln!(out, "     Source:   extra folder {}", m.dir);
        }
    }
    let _ = writeln!(out, "     File:     {} · modified {} · {}", format_bytes(m.size), when(m.mtime), type_name(m.pack_type));
    let _ = writeln!(out, "     Path:     {}", m.path.replace('/', "\\"));
    if let Some(s) = d.se.get(&m.key).and_then(se_line) {
        let _ = writeln!(out, "     Needs:    {s}");
    }
}

pub fn render(d: &ReportData) -> String {
    let mut out = String::new();
    let enabled = ops::enabled_packs(d.profile, d.scan);
    let by_key: HashMap<&str, &ModEntry> = d.scan.iter().map(|m| (m.key.as_str(), m)).collect();

    let _ = writeln!(out, "TK Mod Manager mod report");
    let _ = writeln!(out, "Created:          {}", when(d.now));
    let _ = writeln!(out, "TK Mod Manager:   v{}", ops::APP_VERSION);
    let _ = writeln!(
        out,
        "Game:             {} {} ({})",
        paths::EXE_NAME,
        d.game.exe_version.clone().unwrap_or_else(|| "version unknown".into()),
        d.game.store.label()
    );
    let _ = writeln!(out, "Game folder:      {}", d.game.game_root.clone().unwrap_or_else(|| "not found".into()));
    let _ = writeln!(out, "Workshop folder:  {}", d.game.workshop_dir.clone().unwrap_or_else(|| "none".into()));
    let _ = writeln!(out, "Profile:          {}", d.profile.name);
    let _ = writeln!(
        out,
        "Script extender:  {}",
        match (d.profile.dll, &d.dll) {
            (false, _) => "off for this profile".to_string(),
            (true, Some(v)) => format!("on, DLL v{v}"),
            (true, None) => "on, but no DLL matching this game build is installed".to_string(),
        }
    );
    let _ = writeln!(out, "Skip intro:       {}", if d.profile.skip_intro { "on" } else { "off" });
    let _ = writeln!(
        out,
        "Steam details:    {}",
        match &d.workshop_live {
            Ok(()) => "fetched from Steam just now".to_string(),
            Err(e) => format!("from the manager's cache (Steam could not be asked: {e})"),
        }
    );
    let count = |s: ModSource| enabled.iter().filter(|m| m.source == s).count();
    let _ = writeln!(
        out,
        "Enabled packs:    {} ({} Workshop, {} data folder, {} Nexus, {} extra folder)",
        enabled.len(),
        count(ModSource::Workshop),
        count(ModSource::Data),
        count(ModSource::Nexus),
        count(ModSource::Folder)
    );

    // Things worth a second look, first, so they are not lost under a long list.
    let mut notes: Vec<String> = Vec::new();
    for m in &enabled {
        if ws_gone(m, d) {
            notes.push(format!("{}: Workshop item {} is not public on Steam any more; other players cannot get this version.", m.file, m.workshop_id.clone().unwrap_or_default()));
        } else if ws_pending(m, d.workshop) {
            notes.push(format!("{}: Steam has a newer version than the installed one (let Steam finish downloading, or use Force update).", m.file));
        }
        if m.pack_type == PackType::Movie {
            notes.push(format!("{}: movie pack; the game loads it no matter where it sits in the load order.", m.file));
        }
        if let Some(nx) = m.nexus.as_ref().and_then(|n| n.newer.as_ref()) {
            notes.push(format!("{}: a newer file is on Nexus Mods ({} v{}).", m.file, nx.name, nx.version));
        }
        // Requirements the author declared on the Workshop page ("Required items").
        if let Some(w) = m.workshop_id.as_ref().and_then(|id| d.workshop.get(id)) {
            for req in &w.required_items {
                if enabled.iter().any(|e| e.workshop_id.as_deref() == Some(req.as_str())) {
                    continue;
                }
                let name = d.workshop.get(req).map(|r| r.title.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| "title unknown".into());
                let state = if d.scan.iter().any(|s| s.workshop_id.as_deref() == Some(req.as_str())) { "installed but not enabled" } else { "not installed" };
                notes.push(format!("{}: requires Workshop item {req} ({name}), which is {state}.", m.file));
            }
        }
        if d.se.get(&m.key).map(|s| s.required).unwrap_or(false) && !d.profile.dll {
            notes.push(format!("{}: needs the script extender, which is off for this profile.", m.file));
        }
    }
    for c in ops::name_clashes(d.profile, d.scan) {
        let others: Vec<String> = c.others.iter().map(|o| format!("{}{}", o.path.replace('/', "\\"), if o.newer { " (newer)" } else { "" })).collect();
        notes.push(format!("{}: more than one copy on disk; the game gets the name only. Loaded: {}. Also: {}", c.file, by_key.get(c.loaded.as_str()).map(|m| m.path.as_str()).unwrap_or(&c.loaded), others.join("; ")));
    }
    let missing: Vec<&str> = d
        .profile
        .entries
        .iter()
        .filter(|e| e.enabled && !e.is_separator() && !by_key.contains_key(e.key.as_str()))
        .map(|e| e.key.as_str())
        .collect();
    for k in &missing {
        notes.push(format!("{k}: enabled in the profile but not installed, so it is not loaded."));
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "== Check these ({}) ==", notes.len());
    if notes.is_empty() {
        let _ = writeln!(out, "Nothing unusual found.");
    }
    for n in &notes {
        let _ = writeln!(out, "- {n}");
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "== Load order (first to last; a later pack overrides an earlier one) ==");
    for (i, m) in enabled.iter().enumerate() {
        pack_block(&mut out, i + 1, m, d);
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "== Mod list file given to the game ({}) ==", paths::LIST_FILE_NAME);
    let _ = write!(out, "{}", modlist::build(&ops::list_input_for(d.profile, d.scan)));

    let _ = writeln!(out);
    let _ = writeln!(out, "== Recent launches ==");
    if d.launches.is_empty() {
        let _ = writeln!(out, "none recorded");
    }
    for r in d.launches.iter().take(8) {
        let how = match (r.ended, r.exit_code) {
            (None, _) => "still running or not tracked".to_string(),
            (Some(_), Some(0)) => "exited normally".to_string(),
            (Some(_), Some(c)) => format!("ENDED ABNORMALLY (exit code 0x{c:X})"),
            (Some(_), None) => "exit code unknown".to_string(),
        };
        let _ = writeln!(out, "- {} · {} · {} packs · {how}", when(r.started), r.profile, r.packs.len());
    }

    let enabled_keys: std::collections::HashSet<&str> = enabled.iter().map(|m| m.key.as_str()).collect();
    let mut others: Vec<&ModEntry> = d.scan.iter().filter(|m| !enabled_keys.contains(m.key.as_str())).collect();
    others.sort_by(|a, b| crate::fmt::compare_pack_names(&a.file, &b.file));
    let _ = writeln!(out);
    let _ = writeln!(out, "== Installed but not enabled ({}) ==", others.len());
    for m in others {
        let from = match m.source {
            ModSource::Workshop => format!("Workshop {}", m.workshop_id.clone().unwrap_or_default()),
            _ => source_name(m).to_string(),
        };
        let title = title_of(m, d.workshop);
        let title = if title.is_empty() { String::new() } else { format!(" · {title}") };
        let movie = if m.pack_type == PackType::Movie { " · movie" } else { "" };
        let _ = writeln!(out, "- {}{title} · {from} · {} · modified {}{movie}", m.file, format_bytes(m.size), when(m.mtime));
    }
    for (head, body) in &d.extras {
        let _ = writeln!(out);
        let _ = writeln!(out, "== {head} ==");
        let _ = writeln!(out, "{}", body.trim_end());
    }
    out
}

/// The last `lines` lines of a text file (reads at most the final 64 KB).
fn file_tail(path: &Path, lines: usize) -> Option<String> {
    let text = crate::logs::tail(path, 64 * 1024).ok()?;
    let all: Vec<&str> = text.lines().collect();
    Some(all[all.len().saturating_sub(lines)..].join("\n"))
}

/// The game's own crash dumps (`<user dir>\crash_report\D<date>.mdmp` + `.stack.txt`), newest
/// first. The dump itself is too big for a text report; its path is listed so it can be sent.
fn crash_files(store: paths::GameStore) -> String {
    let Some(dir) = paths::game_user_dir(store).map(|d| d.join("crash_report")) else { return "game user folder not found".into() };
    let Ok(rd) = std::fs::read_dir(&dir) else { return format!("none ({} does not exist)", dir.display()) };
    let mut dumps: Vec<(u64, PathBuf, u64)> = rd
        .flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("mdmp")).unwrap_or(false))
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            let t = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
            Some((t, e.path(), m.len()))
        })
        .collect();
    dumps.sort_by(|a, b| b.0.cmp(&a.0));
    if dumps.is_empty() {
        return format!("none in {}", dir.display());
    }
    let mut out = format!("{} dumps in {}; newest first. If asked, send the newest .mdmp file.\n", dumps.len(), dir.display());
    for (t, p, size) in dumps.iter().take(3) {
        let _ = writeln!(out, "- {} · {} · {}", when(*t), p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), format_bytes(*size));
        // The stack file is one module name per frame; collapse runs of the same module.
        if let Ok(stack) = std::fs::read_to_string(p.with_extension("stack.txt")) {
            let mut runs: Vec<(String, usize)> = Vec::new();
            for l in stack.lines().map(str::trim).filter(|l| !l.is_empty()) {
                match runs.last_mut() {
                    Some((m, n)) if m == l => *n += 1,
                    _ => runs.push((l.to_string(), 1)),
                }
            }
            let s: Vec<String> = runs.iter().map(|(m, n)| if *n > 1 { format!("{m} x{n}") } else { m.clone() }).collect();
            if !s.is_empty() {
                let _ = writeln!(out, "    stack: {}", s.join(" > "));
            }
        }
    }
    out
}

/// Windows "Application Error" events for the game (exception code, faulting module and
/// offset), newest first.
fn windows_errors() -> String {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let exe = std::env::var("SystemRoot").map(|r| PathBuf::from(r).join("System32").join("wevtutil.exe")).unwrap_or_else(|_| PathBuf::from("wevtutil.exe"));
    let run = std::process::Command::new(exe)
        .args(["qe", "Application", "/q:*[System[Provider[@Name='Application Error']]]", "/c:300", "/rd:true", "/f:text"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let text = match run {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(e) => return format!("could not read the Windows event log: {e}"),
    };
    let keep = ["Date:", "Faulting module name", "Exception code", "Fault offset", "Faulting application name"];
    let events: Vec<String> = text
        .split("Event[")
        .filter(|e| e.contains(paths::EXE_NAME))
        .take(5)
        .map(|e| e.lines().map(str::trim).filter(|l| keep.iter().any(|k| l.starts_with(k))).collect::<Vec<_>>().join("\n  "))
        .collect();
    if events.is_empty() {
        return format!("no crash of {} recorded by Windows", paths::EXE_NAME);
    }
    events.iter().map(|e| format!("- {e}")).collect::<Vec<_>>().join("\n")
}

fn extras(ctx: &AppContext, game: &paths::GamePaths) -> Vec<(String, String)> {
    let mut out = vec![
        ("Game crash dumps".to_string(), crash_files(game.store)),
        (format!("Windows crash events for {}", paths::EXE_NAME), windows_errors()),
    ];
    if let Some(gfx) = paths::game_user_dir(game.store).map(|d| d.join("logs").join("gfx.log.txt")).and_then(|p| file_tail(&p, 40)) {
        out.push(("Graphics (gfx.log.txt)".into(), gfx));
    }
    for s in crate::logs::log_sources(ctx) {
        if let Some(t) = file_tail(Path::new(&s.path), 80) {
            out.push((format!("{} · last 80 lines · written {}", s.label, when(s.mtime)), t));
        }
    }
    out
}

/// Gather everything for `profile` (asks Steam for fresh details; blocking) and render it.
pub fn build(ctx: &AppContext, profile: &Profile) -> String {
    let scan = ctx.scan();
    let game = ctx.game_paths();
    let mut ids: Vec<String> = scan.iter().filter(|m| m.source == ModSource::Workshop).filter_map(|m| m.workshop_id.clone()).collect();
    ids.sort();
    ids.dedup();
    let (mut ws, live) = match workshop::workshop_fetch(ctx, &ids, true) {
        Ok(items) => (items, Ok(())),
        Err(e) => (workshop::workshop_cached(), Err(e)),
    };
    // Titles for required items that are not installed, so the report can name them.
    let mut missing: Vec<String> = ws.values().flat_map(|w| w.required_items.iter()).filter(|r| !ws.contains_key(*r)).cloned().collect();
    missing.sort();
    missing.dedup();
    if live.is_ok() && !missing.is_empty() {
        if let Ok(more) = workshop::workshop_fetch(ctx, &missing, false) {
            ws.extend(more);
        }
    }
    let keys: Vec<String> = ops::enabled_packs(profile, &scan).iter().map(|m| m.key.clone()).collect();
    let se = if keys.is_empty() { HashMap::new() } else { se_scan::se_requirements(ctx, &keys) };
    let dll = dll::status_for(game.exe.as_deref().map(std::path::Path::new)).selected.map(|d| d.version);
    let launches = history::launch_history();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let extras = extras(ctx, &game);
    render(&ReportData { now, profile, scan: &scan, workshop: &ws, workshop_live: live, se: &se, game: &game, dll, launches: &launches, extras })
}

/// Write the report to the Desktop (or the app folder when there is none) and return its path.
pub fn save(text: &str) -> Result<PathBuf, String> {
    let dir = directories::UserDirs::new().and_then(|u| u.desktop_dir().map(|p| p.to_path_buf())).filter(|p| p.is_dir()).unwrap_or_else(paths::app_data_dir);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, m, d) = ymd(now);
    let s = now % 86_400;
    let path = dir.join(format!("TKMM mod report {y}-{m:02}-{d:02} {:02}{:02}{:02}.txt", s / 3600, s % 3600 / 60, s % 60));
    std::fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/// Discord's upload limit for a webhook message (servers without boosts).
const DISCORD_LIMIT: usize = 10 * 1024 * 1024;

/// The newest crash dump, if the game wrote one in the last `max_age_days` days.
fn recent_dump(store: paths::GameStore, now: u64, max_age_days: u64) -> Option<(PathBuf, u64)> {
    let dir = paths::game_user_dir(store)?.join("crash_report");
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("mdmp")).unwrap_or(false))
        .filter_map(|e| Some((e.path(), e.metadata().ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs())))
        .filter(|(_, t)| now.saturating_sub(*t) <= max_age_days * 86_400)
        .max_by_key(|(_, t)| *t)
}

/// The dump and its stack file, zipped at the strongest level (a 47 MB dump packs to ~10 MB).
fn zip_dump(dump: &Path) -> Result<Vec<u8>, String> {
    use std::io::Write as _;
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_level(Some(9));
    for p in [dump.to_path_buf(), dump.with_extension("stack.txt")] {
        let Ok(bytes) = std::fs::read(&p) else { continue };
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        w.start_file(name, opts).map_err(|e| e.to_string())?;
        w.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    Ok(w.finish().map_err(|e| e.to_string())?.into_inner())
}

/// The lines under the "Check these" heading of a rendered report.
fn checks_of(report: &str) -> Vec<&str> {
    report
        .lines()
        .skip_while(|l| !l.starts_with("== Check these"))
        .skip(1)
        .take_while(|l| !l.starts_with("== "))
        .filter(|l| !l.trim().is_empty())
        .collect()
}

/// Cut to at most `max` characters (Discord counts characters, not bytes).
fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Title and message of the forum post: the player's note, the report header (without the
/// folder lines, which carry the Windows user name) and the "Check these" list.
fn post_text(report: &str, profile: &str, note: &str, dump: &str) -> (String, String) {
    let first = note.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    let title = clip(&if first.is_empty() { format!("Crash report · {profile}") } else { format!("{first} · {profile}") }, 100);
    let head: Vec<&str> = report
        .lines()
        .skip(1)
        .take_while(|l| !l.is_empty())
        .filter(|l| !l.starts_with("Game folder") && !l.starts_with("Workshop folder"))
        .collect();
    let mut body = String::new();
    if !note.trim().is_empty() {
        let _ = writeln!(body, "**What happened:** {}\n", clip(note.trim(), 600));
    }
    let _ = writeln!(body, "```\n{}\n```", head.join("\n"));
    let checks = checks_of(report);
    let _ = writeln!(body, "**Check these ({}):**", checks.iter().filter(|l| l.starts_with("- ")).count());
    for c in checks {
        body.push_str(&clip(c, 160));
        body.push('\n');
    }
    // The dump line goes last but must survive the 2000-character cut.
    let room = 2000usize.saturating_sub(dump.chars().count() + 1);
    let mut body = clip(&body, room);
    if !body.ends_with('\n') {
        body.push('\n');
    }
    body.push_str(dump);
    (title, body)
}

/// Post the report (and the newest crash dump, when Discord accepts the size) as a new thread in
/// a Discord forum channel through `webhook`. `note` is what the player wrote about the crash.
/// Blocking (builds the report and uploads).
pub fn send_to_discord(ctx: &AppContext, profile: &Profile, webhook: &str, note: &str) -> Result<String, String> {
    use reqwest::blocking::multipart::{Form, Part};
    let report = build(ctx, profile);
    let game = ctx.game_paths();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let dump = recent_dump(game.store, now, 7).and_then(|(p, t)| zip_dump(&p).ok().map(|z| (p, t, z)));
    let (y, m, d) = ymd(now);
    let report_name = format!("mod_report_{y}-{m:02}-{d:02}.txt");
    let client = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(300)).build().map_err(|e| e.to_string())?;

    // With the dump first when it could fit; Discord answers 413 when the message is too big,
    // and then the report goes alone.
    let mut attempts: Vec<Option<&(PathBuf, u64, Vec<u8>)>> = Vec::new();
    if let Some(z) = dump.as_ref().filter(|z| z.2.len() + report.len() <= DISCORD_LIMIT) {
        attempts.push(Some(z));
    }
    attempts.push(None);
    let mut last_err = String::new();
    for with in attempts {
        let dump_line = match (with, &dump) {
            (Some((p, t, z)), _) => {
                format!("Crash dump {} from {} attached ({}).", p.file_name().unwrap_or_default().to_string_lossy(), when(*t), format_bytes(z.len() as u64))
            }
            (None, Some((p, t, z))) => format!(
                "Crash dump {} from {} is too big for Discord ({} zipped); ask the player for it.",
                p.file_name().unwrap_or_default().to_string_lossy(),
                when(*t),
                format_bytes(z.len() as u64)
            ),
            (None, None) => "No crash dump from the last 7 days.".to_string(),
        };
        let (title, content) = post_text(&report, &profile.name, note, &dump_line);
        let payload = serde_json::json!({
            "thread_name": title,
            "content": content,
            "allowed_mentions": { "parse": [] },
        });
        let text_part = Part::bytes(report.clone().into_bytes()).file_name(report_name.clone()).mime_str("text/plain").map_err(|e| e.to_string())?;
        let mut form = Form::new().text("payload_json", payload.to_string()).part("files[0]", text_part);
        if let Some((_, _, z)) = with {
            form = form.part("files[1]", Part::bytes(z.clone()).file_name("crash_dump.zip").mime_str("application/zip").map_err(|e| e.to_string())?);
        }
        let resp = client.post(format!("{webhook}?wait=true")).multipart(form).send().map_err(|e| format!("cannot reach Discord: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(if with.is_some() { "Report and crash dump sent.".into() } else { "Report sent.".into() });
        }
        let body = resp.text().unwrap_or_default();
        last_err = format!("Discord refused the report ({status}): {}", clip(&body, 300));
        if status.as_u16() != 413 && !body.contains("40005") {
            break;
        }
    }
    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::ProfileEntry;

    fn pack(key: &str, source: ModSource, ws: Option<&str>, pack_type: PackType, installed: Option<u64>, latest: Option<u64>) -> ModEntry {
        let file = key.rsplit(['/', ':']).next().unwrap().to_string();
        ModEntry {
            key: key.into(),
            file: file.clone(),
            path: format!("C:/x/{file}"),
            dir: "C:/x".into(),
            source,
            workshop_id: ws.map(Into::into),
            pack_type,
            size: 2048,
            mtime: 1_758_000_000,
            preview_path: None,
            installed_updated: installed,
            latest_updated: latest,
            nexus: None,
        }
    }

    fn item(id: &str, title: &str, updated: u64) -> WorkshopItem {
        WorkshopItem {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            time_updated: updated,
            time_created: 0,
            tags: vec![],
            preview_url: String::new(),
            required_items: vec![],
            fetched_at: 1,
            from_launcher_cache: false,
        }
    }

    #[test]
    fn report_lists_order_sources_and_warnings() {
        let scan = vec![
            pack("ws:1/core.pack", ModSource::Workshop, Some("1"), PackType::Mod, Some(100), Some(100)),
            pack("ws:2/old.pack", ModSource::Workshop, Some("2"), PackType::Mod, Some(100), Some(200)),
            pack("ws:3/gone.pack", ModSource::Workshop, Some("3"), PackType::Mod, Some(100), Some(100)),
            pack("ws:4/audio.pack", ModSource::Workshop, Some("4"), PackType::Movie, Some(100), Some(100)),
            pack("data:local.pack", ModSource::Data, None, PackType::Mod, None, None),
            pack("data:spare.pack", ModSource::Data, None, PackType::Mod, None, None),
        ];
        let e = |k: &str| ProfileEntry { key: k.into(), enabled: true, label: None, collapsed: false };
        let profile = Profile {
            name: "Test".into(),
            entries: vec![e("ws:1/core.pack"), e("ws:2/old.pack"), e("ws:3/gone.pack"), e("ws:4/audio.pack"), e("data:local.pack"), e("ws:9/missing.pack")],
            dll: false,
            skip_intro: false,
            last_played: None,
        };
        let ws: HashMap<String, WorkshopItem> =
            [("1", "Core Mod", 100), ("2", "Old Mod", 200), ("4", "Audio", 100)].into_iter().map(|(i, t, u)| (i.to_string(), item(i, t, u))).collect();
        let game = paths::GamePaths::default();
        let se = HashMap::new();
        let text = render(&ReportData { now: 1_758_000_000, profile: &profile, scan: &scan, workshop: &ws, workshop_live: Ok(()), se: &se, game: &game, dll: None, launches: &[], extras: vec![] });

        let pos = |s: &str| text.find(s).unwrap_or_else(|| panic!("missing {s:?} in\n{text}"));
        assert!(pos("  1. core.pack") < pos("  2. old.pack"));
        assert!(text.contains("Title:    Core Mod"));
        assert!(text.contains("https://steamcommunity.com/sharedfiles/filedetails/?id=1"));
        assert!(text.contains("old.pack: Steam has a newer version"));
        assert!(text.contains("gone.pack: Workshop item 3 is not public"));
        assert!(text.contains("audio.pack: movie pack"));
        assert!(text.contains("ws:9/missing.pack: enabled in the profile but not installed"));
        assert!(text.contains("game data folder (copied in by hand"));
        // Not enabled, listed separately; the movie pack gets no `mod` line in the list file.
        assert!(pos("== Installed but not enabled (1) ==") < pos("- spare.pack"));
        assert!(!text.contains("mod \"audio.pack\";"));
        assert!(text.contains("mod \"core.pack\";"));
    }

    #[test]
    fn offline_report_does_not_call_items_removed() {
        let scan = vec![pack("ws:3/gone.pack", ModSource::Workshop, Some("3"), PackType::Mod, Some(100), Some(100))];
        let profile = Profile {
            name: "P".into(),
            entries: vec![ProfileEntry { key: "ws:3/gone.pack".into(), enabled: true, label: None, collapsed: false }],
            dll: false,
            skip_intro: false,
            last_played: None,
        };
        let (ws, se, game) = (HashMap::new(), HashMap::new(), paths::GamePaths::default());
        let text = render(&ReportData { now: 0, profile: &profile, scan: &scan, workshop: &ws, workshop_live: Err("offline".into()), se: &se, game: &game, dll: None, launches: &[], extras: vec![] });
        assert!(!text.contains("not public"));
        assert!(text.contains("Steam could not be asked: offline"));
    }

    #[test]
    fn post_text_fits_discord_limits() {
        let report = "TK Mod Manager mod report\nCreated:          x\nGame folder:      C:/Users/kyle/game\nProfile:          P\n\n== Check these (2) ==\n- a.pack: movie pack\n- b.pack: requires Workshop item 1\n\n== Load order ==\n  1. a.pack\n";
        let (title, body) = post_text(report, "P", &"x".repeat(3000), "No crash dump.");
        assert!(title.chars().count() <= 100);
        assert!(body.chars().count() <= 2000);
        assert!(body.ends_with("No crash dump."));
        assert!(body.contains("- b.pack: requires"));
        assert!(body.contains("**Check these (2):**"));
        assert!(!body.contains("kyle"), "folder lines stay out of the post");
        let (title, _) = post_text(report, "Basic", "", "");
        assert_eq!(title, "Crash report · Basic");
        // A long warning list is cut, never the dump line after it.
        let many = report.replace("- b.pack", &format!("{}- b.pack", "- c.pack: more than one copy on disk\n".repeat(80)));
        assert!(checks_of(&many).len() > 80);
        let (_, body) = post_text(&many, "P", "", "Crash dump attached.");
        assert!(body.chars().count() <= 2000 && body.ends_with("Crash dump attached."));
    }

    #[test]
    fn dates_are_utc_minutes() {
        assert_eq!(when(0), "unknown");
        assert_eq!(when(1_758_000_000), "2025-09-16 05:20 UTC");
    }
}
