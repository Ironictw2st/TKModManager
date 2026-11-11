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
    /// Three Kingdoms Coop in the game folder (`coop::CoopInstall::summary`), None = not installed.
    pub coop: Option<String>,
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

/// Steam's public API returned nothing for the id although we asked it just now: the item is
/// unlisted, private or removed. Unlisted items still download for subscribers.
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
            let state = if ws_pending(m, d.workshop) {
                "UPDATE PENDING: Steam has a newer version than the one installed"
            } else if ws_gone(m, d) {
                "not publicly listed on Steam (unlisted, private or removed)"
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
    let _ = writeln!(out, "Coop (3-4 MP):    {}", d.coop.as_deref().unwrap_or("not installed"));
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
        if ws_pending(m, d.workshop) {
            notes.push(format!(
                "{}: OUT OF DATE: installed version from {}, newest on Steam from {} (let Steam finish downloading, or use Force update).",
                m.file,
                when(m.installed_updated.unwrap_or(0)),
                when(ws_latest(m, d.workshop))
            ));
        }
        if ws_gone(m, d) {
            notes.push(format!(
                "{}: Workshop item {} is not publicly listed (unlisted test build, private or removed); check it is the build the player should have.",
                m.file,
                m.workshop_id.clone().unwrap_or_default()
            ));
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
fn crash_files(store: paths::GameStore, cc: &CrashContext) -> String {
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
    for (i, (t, p, size)) in dumps.iter().take(3).enumerate() {
        let _ = writeln!(out, "- {} · {} · {}", when(*t), p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), format_bytes(*size));
        if i == 0 {
            match cc.dump_match(*t) {
                DumpMatch::Matches => {
                    let _ = writeln!(out, "    belongs to the latest crash");
                }
                DumpMatch::Earlier(why) => {
                    let _ = writeln!(out, "    FROM AN EARLIER CRASH ({why}); the latest crash has no dump");
                }
                DumpMatch::Unverified => {}
            }
        }
        // Read on this PC, so the crash can be triaged even when the dump is too big to send.
        if i == 0 {
            for l in crate::crash_dump::summary_text(p).lines() {
                let _ = writeln!(out, "    {l}");
            }
        }
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

/// One Windows "Application Error" event (id 1000) for the game.
#[derive(Clone, Debug, PartialEq)]
struct CrashEvent {
    /// Unix seconds, UTC (from the event's SystemTime; the text form of wevtutil prints local
    /// time with a "Z", so it is not used).
    when: u64,
    app_version: String,
    app_stamp: String,
    module: String,
    module_version: String,
    code: String,
    offset: String,
}

impl CrashEvent {
    /// "Three_Kingdoms.exe+0x2326aa4" (leading zeros of the offset dropped).
    fn location(&self) -> String {
        let hex = self.offset.trim().trim_start_matches("0x").trim_start_matches('0');
        format!("{}+0x{}", self.module, if hex.is_empty() { "0" } else { hex })
    }
}

/// "2026-11-08T04:03:09.1760000Z" -> unix seconds.
fn parse_system_time(s: &str) -> Option<u64> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-').map(|x| x.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let mut t = time.trim_end_matches('Z').split(':');
    let (h, mi) = (t.next()?.parse::<u64>().ok()?, t.next()?.parse::<u64>().ok()?);
    let sec = t.next()?.split('.').next()?.parse::<u64>().ok()?;
    Some(crate::fmt::days_from_civil(y, m as u32, day as u32) + h * 3600 + mi * 60 + sec)
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// The game's events from `wevtutil qe ... /f:xml` output, in the order given (newest first).
fn parse_crash_events(xml: &str) -> Vec<CrashEvent> {
    xml.split("<Event ")
        .skip(1)
        .filter_map(|e| {
            let st = e.split("SystemTime='").nth(1)?.split('\'').next()?;
            let data: Vec<String> = e
                .split("<Data>")
                .skip(1)
                .map(|d| xml_unescape(d.split("</Data>").next().unwrap_or("")))
                .collect();
            // Event 1000 data: app, app version, app stamp, module, module version, module stamp,
            // exception code, fault offset, ...
            if data.len() < 8 || !data[0].eq_ignore_ascii_case(paths::EXE_NAME) {
                return None;
            }
            Some(CrashEvent {
                when: parse_system_time(st)?,
                app_version: data[1].clone(),
                app_stamp: data[2].clone(),
                module: data[3].clone(),
                module_version: data[4].clone(),
                code: data[6].clone(),
                offset: data[7].clone(),
            })
        })
        .take(5)
        .collect()
}

/// Windows "Application Error" events for the game (exception code, faulting module and
/// offset), newest first.
fn windows_crash_events() -> Result<Vec<CrashEvent>, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let exe = std::env::var("SystemRoot").map(|r| PathBuf::from(r).join("System32").join("wevtutil.exe")).unwrap_or_else(|_| PathBuf::from("wevtutil.exe"));
    let run = std::process::Command::new(exe)
        .args(["qe", "Application", "/q:*[System[Provider[@Name='Application Error']]]", "/c:300", "/rd:true", "/f:xml"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match run {
        Ok(o) => Ok(parse_crash_events(&String::from_utf8_lossy(&o.stdout))),
        Err(e) => Err(format!("could not read the Windows event log: {e}")),
    }
}

fn render_crash_events(events: &Result<Vec<CrashEvent>, String>) -> String {
    let events = match events {
        Ok(e) => e,
        Err(e) => return e.clone(),
    };
    if events.is_empty() {
        return format!("no crash of {} recorded by Windows", paths::EXE_NAME);
    }
    events
        .iter()
        .map(|e| {
            format!(
                "- Date: {}\n  Faulting application name: {}, version: {}, time stamp: 0x{}\n  Faulting module name: {}, version: {}\n  Exception code: {}\n  Fault offset: {} ({})",
                when(e.when),
                paths::EXE_NAME,
                e.app_version,
                e.app_stamp,
                e.module,
                e.module_version,
                e.code,
                e.offset,
                e.location()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether the newest crash dump belongs to the crash being reported.
#[derive(Clone, Debug, PartialEq)]
enum DumpMatch {
    /// Written next to the newest Windows crash event, or during the last launch.
    Matches,
    /// Older than the newest crash (or the last launch): it is from an earlier crash.
    Earlier(String),
    /// Nothing to compare with (no Windows event, no recorded launch).
    Unverified,
}

/// The game's reporter writes its dump a few seconds before Windows logs the crash; allow
/// 10 minutes before and 2 after.
const DUMP_BEFORE_EVENT: u64 = 600;
const DUMP_AFTER_EVENT: u64 = 120;

fn match_dump(dump: u64, newest_crash: Option<u64>, last_launch: Option<u64>) -> DumpMatch {
    if let Some(c) = newest_crash {
        if dump + DUMP_BEFORE_EVENT >= c && dump <= c + DUMP_AFTER_EVENT {
            return DumpMatch::Matches;
        }
        if dump < c {
            return DumpMatch::Earlier(format!("Windows recorded a newer crash at {}", when(c)));
        }
    }
    match last_launch {
        Some(l) if dump + 60 >= l => DumpMatch::Matches,
        Some(l) => DumpMatch::Earlier(format!("it is older than the last launch at {}", when(l))),
        None => DumpMatch::Unverified,
    }
}

/// What this PC knows about the most recent crash: Windows events and the last launch start.
struct CrashContext {
    events: Result<Vec<CrashEvent>, String>,
    last_launch: Option<u64>,
}

impl CrashContext {
    fn gather(launches: &[history::LaunchRecord]) -> Self {
        CrashContext { events: windows_crash_events(), last_launch: launches.first().map(|r| r.started) }
    }
    fn newest(&self) -> Option<&CrashEvent> {
        self.events.as_ref().ok().and_then(|e| e.first())
    }
    fn dump_match(&self, dump: u64) -> DumpMatch {
        match_dump(dump, self.newest().map(|e| e.when), self.last_launch)
    }
}

/// `se_crash.txt` next to the script extender DLL. The DLL appends to it every session, so only
/// a file written since the last launch (or near the newest crash) counts, and only its end is
/// sent.
const SE_CRASH_TAIL: u64 = 256 * 1024;

fn se_crash_file(dll_dir: Option<&Path>, cc: &CrashContext, now: u64) -> Option<(PathBuf, u64, u64)> {
    let path = dll_dir?.join("se_crash.txt");
    let meta = std::fs::metadata(&path).ok()?;
    let t = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
    let recent = match (cc.last_launch, cc.newest()) {
        (Some(l), _) => t + 60 >= l,
        (None, Some(e)) => t + DUMP_BEFORE_EVENT >= e.when,
        (None, None) => now.saturating_sub(t) <= 86_400,
    };
    recent.then_some((path, t, meta.len()))
}

fn se_crash_tail(path: &Path) -> Option<Vec<u8>> {
    crate::logs::tail(path, SE_CRASH_TAIL).ok().map(String::into_bytes)
}

fn extras(ctx: &AppContext, game: &paths::GamePaths, cc: &CrashContext, se_crash: Option<&(PathBuf, u64, u64)>) -> Vec<(String, String)> {
    let mut out = vec![
        ("Game crash dumps".to_string(), crash_files(game.store, cc)),
        (format!("Windows crash events for {}", paths::EXE_NAME), render_crash_events(&cc.events)),
    ];
    let se_text = match se_crash {
        Some((p, t, size)) => format!("{} · written {} · {} (its end is attached when the report is sent)", p.display(), when(*t), format_bytes(*size)),
        None => "none written since the last launch".to_string(),
    };
    out.push(("Script extender crash report (se_crash.txt)".to_string(), se_text));
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
    let launches = history::launch_history();
    let cc = CrashContext::gather(&launches);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let se_crash = se_crash_file(selected_dll_dir(ctx).as_deref(), &cc, now);
    build_with(ctx, profile, &launches, &cc, se_crash.as_ref())
}

/// Folder of the script extender DLL a launch would inject.
fn selected_dll_dir(ctx: &AppContext) -> Option<PathBuf> {
    let game = ctx.game_paths();
    dll::status_for(game.exe.as_deref().map(std::path::Path::new)).selected.map(|d| PathBuf::from(d.dir))
}

fn build_with(ctx: &AppContext, profile: &Profile, launches: &[history::LaunchRecord], cc: &CrashContext, se_crash: Option<&(PathBuf, u64, u64)>) -> String {
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
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let extras = extras(ctx, &game, cc, se_crash);
    let coop = game.game_root.as_deref().and_then(|r| crate::coop::detect(std::path::Path::new(r))).map(|c| c.summary());
    render(&ReportData { now, profile, scan: &scan, workshop: &ws, workshop_live: live, se: &se, game: &game, dll, coop, launches, extras })
}

/// Write the report to the Desktop (or the app folder when there is none) and return its path.
pub fn save(text: &str) -> Result<PathBuf, String> {
    let dir = directories::UserDirs::new().and_then(|u| u.desktop_dir().map(|p| p.to_path_buf())).filter(|p| p.is_dir()).unwrap_or_else(paths::app_data_dir);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, m, d) = ymd(now);
    let s = now % 86_400;
    // The time is UTC; the name says so, as it can be hours off the player's clock.
    let path = dir.join(format!("TKMM mod report {y}-{m:02}-{d:02} {:02}{:02}{:02} UTC.txt", s / 3600, s % 3600 / 60, s % 60));
    std::fs::write(&path, text).map_err(|e| crate::t!("err-write-file", path = path.display().to_string(), error = e.to_string()))?;
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

/// Largest attachment per follow-up message (Discord's limit is 10 MiB; this leaves headroom).
const PART_SIZE: usize = 9_500_000;
/// Most parts a dump is split into; a bigger one is left on the player's PC (summary only).
const MAX_PARTS: usize = 6;
/// Room kept in the first message for the report text and the form around it.
const MESSAGE_SLACK: usize = 64 * 1024;

/// How long to wait before retrying after Discord's 429: its `retry_after` seconds (JSON body)
/// or the `Retry-After` header, at most 30 s, at least 100 ms.
fn retry_delay(body: &str, header: Option<&str>) -> std::time::Duration {
    let secs = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["retry_after"].as_f64())
        .or_else(|| header.and_then(|h| h.trim().parse::<f64>().ok()))
        .unwrap_or(1.0);
    std::time::Duration::from_secs_f64(secs.clamp(0.1, 30.0))
}

/// POST a multipart form (rebuilt for each try), waiting and retrying while Discord answers 429.
/// Returns the final status and body.
fn post_form(
    client: &reqwest::blocking::Client,
    url: &str,
    form: &dyn Fn() -> Result<reqwest::blocking::multipart::Form, String>,
) -> Result<(reqwest::StatusCode, String), String> {
    let mut tries = 0;
    loop {
        let resp = client.post(url).multipart(form()?).send().map_err(|e| crate::t!("report-unreachable", error = e.to_string()))?;
        let status = resp.status();
        let header = resp.headers().get("retry-after").and_then(|v| v.to_str().ok()).map(str::to_string);
        let body = resp.text().unwrap_or_default();
        tries += 1;
        if status.as_u16() == 429 && tries < 5 {
            std::thread::sleep(retry_delay(&body, header.as_deref()));
            continue;
        }
        return Ok((status, body));
    }
}

/// The id of the forum thread a webhook post created (the message's channel).
fn thread_id_of(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body).ok()?["channel_id"].as_str().map(str::to_string)
}

/// Post parts 2.. of the dump into the thread, one message each.
fn post_follow_ups(client: &reqwest::blocking::Client, webhook: &str, thread: &str, base: &str, parts: &[&[u8]]) -> Result<(), String> {
    use reqwest::blocking::multipart::{Form, Part};
    let url = format!("{webhook}?thread_id={thread}&wait=true");
    for (i, bytes) in parts.iter().enumerate().skip(1) {
        let name = crate::crash_dump::part_name(base, i, parts.len());
        let payload = serde_json::json!({
            "content": format!("Crash dump part {} of {} ({name}, {}).", i + 1, parts.len(), format_bytes(bytes.len() as u64)),
            "allowed_mentions": { "parse": [] },
        });
        let build = || -> Result<Form, String> {
            let part = Part::bytes(bytes.to_vec()).file_name(name.clone()).mime_str("application/octet-stream").map_err(|e| e.to_string())?;
            Ok(Form::new().text("payload_json", payload.to_string()).part("files[0]", part))
        };
        let (status, body) = post_form(client, &url, &build)?;
        if !status.is_success() {
            return Err(crate::t!("report-refused", status = status.to_string(), message = clip(&body, 300)));
        }
    }
    Ok(())
}

/// What the first message says about the dump.
fn dump_line(sig: Option<&str>, name: &str, when_text: &str, packed: Option<(&crate::crash_dump::Packed, usize, bool)>) -> String {
    let mut out = String::new();
    if let Some(s) = sig {
        let _ = writeln!(out, "{s}");
    }
    match packed {
        Some((z, 1, true)) => {
            let _ = write!(out, "Crash dump {name} from {when_text} attached ({} {}).", format_bytes(z.bytes.len() as u64), z.method);
        }
        Some((z, n, true)) => {
            let _ = write!(
                out,
                "Crash dump {name} from {when_text}: {} {} in {n} parts, posted below. Join: `{}` (7-Zip also opens .001 directly).",
                format_bytes(z.bytes.len() as u64),
                z.method,
                crate::crash_dump::join_command(z.name, n)
            );
        }
        Some((z, _, false)) => {
            let _ = write!(out, "Crash dump {name} from {when_text} is too big for Discord ({} {}); ask the player for it.", format_bytes(z.bytes.len() as u64), z.method);
        }
        None => out.push_str("No crash dump from the last 7 days."),
    }
    out
}

/// Lines of the first message about the latest crash, above the dump line: where Windows saw it
/// crash, why an old dump was left out, and whether `se_crash.txt` is attached.
fn crash_lines(newest: Option<&CrashEvent>, earlier: Option<(&str, &str, &str)>, se_attached: bool) -> String {
    let mut out = String::new();
    if let Some(e) = newest {
        let _ = writeln!(out, "**Latest crash (Windows):** {} · {} · {}", e.location(), e.code, when(e.when));
    }
    if let Some((name, t, why)) = earlier {
        let _ = writeln!(out, "No crash dump for this crash: the newest dump ({name}, {t}) is from an earlier crash; {why}.");
    }
    if se_attached {
        let _ = writeln!(out, "Script extender crash report attached (se_crash.txt).");
    }
    out
}

/// Post the report (and the newest crash dump) as a new thread in a Discord forum channel
/// through `webhook`. `note` is what the player wrote about the crash. A dump that does not fit
/// one message is packed as small as possible (zip, or 7z when that is smaller) and posted in
/// parts as follow-ups in the same thread; one that needs more than `MAX_PARTS` is left out (the
/// crash summary in the report still says where it crashed). Blocking.
pub fn send_to_discord(ctx: &AppContext, profile: &Profile, webhook: &str, note: &str) -> Result<String, String> {
    use crate::crash_dump::{pack, part_name, split_ranges};
    use reqwest::blocking::multipart::{Form, Part};
    let launches = history::launch_history();
    let cc = CrashContext::gather(&launches);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let se_file = se_crash_file(selected_dll_dir(ctx).as_deref(), &cc, now);
    let report = build_with(ctx, profile, &launches, &cc, se_file.as_ref());
    let se_bytes = se_file.as_ref().and_then(|(p, _, _)| se_crash_tail(p));
    let game = ctx.game_paths();
    let first_cap = DISCORD_LIMIT.saturating_sub(report.len() + se_bytes.as_ref().map_or(0, Vec::len) + MESSAGE_SLACK).min(PART_SIZE);
    // Only a dump that belongs to the latest crash is sent; an older one would be mistaken for it.
    let mut earlier: Option<(String, String, String)> = None;
    let dump = recent_dump(game.store, now, 7)
        .filter(|(p, t)| match cc.dump_match(*t) {
            DumpMatch::Earlier(why) => {
                earlier = Some((p.file_name().unwrap_or_default().to_string_lossy().into_owned(), when(*t), why));
                false
            }
            _ => true,
        })
        .and_then(|(p, t)| pack(&p, first_cap).ok().map(|z| (p, t, z)));
    let newest = cc.newest().filter(|e| now.saturating_sub(e.when) <= 7 * 86_400);
    let head = crash_lines(newest, earlier.as_ref().map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str())), se_bytes.is_some());
    let ranges = dump.as_ref().and_then(|(_, _, z)| split_ranges(z.bytes.len(), first_cap, PART_SIZE, MAX_PARTS));
    let (y, m, d) = ymd(now);
    let report_name = format!("mod_report_{y}-{m:02}-{d:02}.txt");
    // The signature is read from the newest dump; it describes this crash only when that dump is sent.
    let sig = dump.as_ref().and_then(|_| report.lines().find(|l| l.trim_start().starts_with("Crash signature:")).map(|l| l.trim().to_string()));
    let client = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(300)).build().map_err(|e| e.to_string())?;

    // With the dump first when it could fit; Discord answers 413 when the message is too big,
    // and then the report goes alone.
    let mut attempts = vec![ranges.is_some()];
    if ranges.is_some() {
        attempts.push(false);
    }
    let mut last_err = String::new();
    for with in attempts {
        let (name, when_text) = dump
            .as_ref()
            .map(|(p, t, _)| (p.file_name().unwrap_or_default().to_string_lossy().into_owned(), when(*t)))
            .unwrap_or_default();
        let mut line = head.clone();
        if dump.is_some() || earlier.is_none() {
            line.push_str(&dump_line(sig.as_deref(), &name, &when_text, dump.as_ref().map(|(_, _, z)| (z, ranges.as_ref().map_or(0, |r| r.len()), with))));
        }
        let line = line.trim_end().to_string();
        let (title, content) = post_text(&report, &profile.name, note, &line);
        let payload = serde_json::json!({
            "thread_name": title,
            "content": content,
            "allowed_mentions": { "parse": [] },
        });
        let first_part: Option<(&[u8], String)> = match (with, &dump, &ranges) {
            (true, Some((_, _, z)), Some(r)) => Some((&z.bytes[r[0].clone()], part_name(z.name, 0, r.len()))),
            _ => None,
        };
        let build = || -> Result<Form, String> {
            let text_part = Part::bytes(report.clone().into_bytes()).file_name(report_name.clone()).mime_str("text/plain").map_err(|e| e.to_string())?;
            let mut form = Form::new().text("payload_json", payload.to_string()).part("files[0]", text_part);
            let mut next = 1;
            if let Some(se) = &se_bytes {
                form = form.part("files[1]", Part::bytes(se.clone()).file_name("se_crash.txt").mime_str("text/plain").map_err(|e| e.to_string())?);
                next = 2;
            }
            if let Some((bytes, fname)) = &first_part {
                form = form.part(format!("files[{next}]"), Part::bytes(bytes.to_vec()).file_name(fname.clone()).mime_str("application/octet-stream").map_err(|e| e.to_string())?);
            }
            Ok(form)
        };
        let (status, body) = post_form(&client, &format!("{webhook}?wait=true"), &build)?;
        if status.is_success() {
            let (Some((_, _, z)), Some(r), true) = (&dump, &ranges, with) else {
                return Ok(if dump.is_some() { crate::t!("report-sent-dump-too-big") } else { crate::t!("report-sent") });
            };
            if r.len() == 1 {
                return Ok(crate::t!("report-sent-with-dump"));
            }
            let parts: Vec<&[u8]> = r.iter().map(|x| &z.bytes[x.clone()]).collect();
            let Some(thread) = thread_id_of(&body) else {
                return Ok(crate::t!("report-parts-failed", part = "2", error = "Discord did not return the thread id".to_string()));
            };
            return match post_follow_ups(&client, webhook, &thread, z.name, &parts) {
                Ok(()) => Ok(crate::t!("report-sent-with-parts", parts = r.len().to_string())),
                Err(e) => Ok(crate::t!("report-parts-failed", part = "2+", error = e)),
            };
        }
        last_err = crate::t!("report-refused", status = status.to_string(), message = clip(&body, 300));
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
            pack("ws:3/gone.pack", ModSource::Workshop, Some("3"), PackType::Mod, Some(100), Some(300)),
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
            se_config: None,
        };
        let ws: HashMap<String, WorkshopItem> =
            [("1", "Core Mod", 100), ("2", "Old Mod", 200), ("4", "Audio", 100)].into_iter().map(|(i, t, u)| (i.to_string(), item(i, t, u))).collect();
        let game = paths::GamePaths::default();
        let se = HashMap::new();
        let text = render(&ReportData { now: 1_758_000_000, profile: &profile, scan: &scan, workshop: &ws, workshop_live: Ok(()), se: &se, game: &game, dll: None, coop: None, launches: &[], extras: vec![] });

        let pos = |s: &str| text.find(s).unwrap_or_else(|| panic!("missing {s:?} in\n{text}"));
        assert!(pos("  1. core.pack") < pos("  2. old.pack"));
        assert!(text.contains("Title:    Core Mod"));
        assert!(text.contains("https://steamcommunity.com/sharedfiles/filedetails/?id=1"));
        assert!(text.contains("old.pack: OUT OF DATE"));
        assert!(text.contains("gone.pack: Workshop item 3 is not publicly listed"));
        assert!(text.contains("gone.pack: OUT OF DATE: installed version from 1970-01-01 00:01 UTC, newest on Steam from 1970-01-01 00:05 UTC"));
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
            se_config: None,
        };
        let (ws, se, game) = (HashMap::new(), HashMap::new(), paths::GamePaths::default());
        let text = render(&ReportData { now: 0, profile: &profile, scan: &scan, workshop: &ws, workshop_live: Err("offline".into()), se: &se, game: &game, dll: None, coop: None, launches: &[], extras: vec![] });
        assert!(!text.contains("not publicly listed"));
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
    fn post_text_keeps_signature_and_join_line_within_limits() {
        let sig = "Crash signature: 0xC0000005 EXCEPTION_ACCESS_VIOLATION_READ at Three_Kingdoms.exe+0x123456";
        let z = crate::crash_dump::Packed { name: "crash_dump.7z", method: "7z", bytes: vec![0; 3] };
        let line = dump_line(Some(sig), "D2026.mdmp", "2026-11-03 12:00", Some((&z, 6, true)));
        assert!(line.starts_with(sig));
        assert!(line.contains("copy /b crash_dump.7z.001+crash_dump.7z.002+crash_dump.7z.003+crash_dump.7z.004+crash_dump.7z.005+crash_dump.7z.006 crash_dump.7z"));
        let report = format!("x
Mod manager 1

== Check these ==
{}
", "- a warning that is rather long indeed
".repeat(200));
        let (_, body) = post_text(&report, "P", &"n".repeat(900), &line);
        assert!(body.chars().count() <= 2000, "{}", body.chars().count());
        assert!(body.ends_with("(7-Zip also opens .001 directly)."));
        assert!(body.contains(sig));
        // Too big to send at all: the signature is still there.
        let line = dump_line(Some(sig), "D.mdmp", "t", Some((&z, 0, false)));
        assert!(line.contains("ask the player") && line.contains(sig));
        assert_eq!(dump_line(None, "", "", None), "No crash dump from the last 7 days.");
    }

    #[test]
    fn retry_delay_honours_discord() {
        use std::time::Duration;
        assert_eq!(retry_delay(r#"{"retry_after": 2.5}"#, None), Duration::from_secs_f64(2.5));
        assert_eq!(retry_delay("", Some("3")), Duration::from_secs(3));
        assert_eq!(retry_delay(r#"{"retry_after": 900}"#, None), Duration::from_secs(30));
        assert_eq!(retry_delay("garbage", None), Duration::from_secs(1));
        assert_eq!(thread_id_of(r#"{"id":"1","channel_id":"42"}"#).as_deref(), Some("42"));
    }

    /// A one-shot HTTP server on localhost: answers each request with the next canned reply and
    /// hands back the request lines + bodies it saw.
    fn mock_server(replies: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<(String, usize)>>) {
        use std::io::{Read, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/hook", listener.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for (code, body) in replies {
                let (mut s, _) = listener.accept().unwrap();
                let mut buf = Vec::new();
                let mut chunk = [0u8; 8192];
                let (head_end, len) = loop {
                    let n = s.read(&mut chunk).unwrap();
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..p]).to_lowercase();
                        let len = head.lines().find_map(|l| l.strip_prefix("content-length:")).and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(0);
                        break (p + 4, if head.contains("transfer-encoding: chunked") { usize::MAX } else { len });
                    }
                };
                // Content-Length bodies are read to the end; chunked ones until the 0-chunk.
                while if len == usize::MAX { !buf.ends_with(b"0\r\n\r\n") } else { buf.len() < head_end + len } {
                    let n = s.read(&mut chunk).unwrap();
                    assert!(n > 0, "client hung up early");
                    buf.extend_from_slice(&chunk[..n]);
                }
                let len = buf.len() - head_end;
                seen.push((String::from_utf8_lossy(&buf).lines().next().unwrap_or("").to_string(), len));
                let reason = if code == 429 { "Too Many Requests" } else { "OK" };
                let head = format!("HTTP/1.1 {code} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = s.write_all(head.as_bytes());
                let _ = s.write_all(body.as_bytes());
            }
            seen
        });
        (url, h)
    }

    #[test]
    fn follow_up_parts_go_to_the_thread_and_wait_out_a_429() {
        let (url, server) = mock_server(vec![
            (429, r#"{"retry_after": 0.1}"#.into()),
            (200, "{}".into()),
            (200, "{}".into()),
        ]);
        let client = reqwest::blocking::Client::builder().no_proxy().build().unwrap();
        let (p1, p2, p3) = (vec![1u8; 10], vec![2u8; 2000], vec![3u8; 30]);
        post_follow_ups(&client, &url, "777", "crash_dump.zip", &[&p1, &p2, &p3]).unwrap();
        let seen = server.join().unwrap();
        // Part 1 went with the first message, so only 2 and 3 are posted (2 after one 429).
        assert_eq!(seen.len(), 3);
        assert!(seen.iter().all(|(l, _)| l.starts_with("POST /hook?thread_id=777&wait=true")), "{seen:?}");
        assert!(seen[1].1 > 2000 && seen[2].1 < seen[1].1);
    }

    #[test]
    fn a_refused_part_is_an_error() {
        let (url, server) = mock_server(vec![(413, r#"{"message":"too big","code":40005}"#.into())]);
        let client = reqwest::blocking::Client::builder().no_proxy().build().unwrap();
        let e = post_follow_ups(&client, &url, "1", "crash_dump.zip", &[&[0u8; 1], &[0u8; 5]]).unwrap_err();
        server.join().unwrap();
        assert!(e.contains("413"), "{e}");
    }

    #[test]
    fn dates_are_utc_minutes() {
        assert_eq!(when(0), "unknown");
        assert_eq!(when(1_758_000_000), "2025-09-16 05:20 UTC");
    }

    const EVENT_XML: &str = "<Event xmlns='x'><System><Provider Name='Application Error'/><EventID>1000</EventID><TimeCreated SystemTime='2026-11-08T04:03:09.1760000Z'/></System><EventData><Data>Three_Kingdoms.exe</Data><Data>1.7.6.0</Data><Data>69ce4df8</Data><Data>Three_Kingdoms.exe</Data><Data>1.7.6.0</Data><Data>69ce4df8</Data><Data>c0000005</Data><Data>0000000002326aa4</Data><Data>1</Data></EventData></Event><Event xmlns='x'><System><TimeCreated SystemTime='2026-11-08T04:00:00.0Z'/></System><EventData><Data>GCC.exe</Data><Data>1</Data><Data>2</Data><Data>KERNELBASE.dll</Data><Data>1</Data><Data>2</Data><Data>e0434352</Data><Data>25369</Data></EventData></Event><Event xmlns='x'><System><TimeCreated SystemTime='2026-11-08T03:36:57.082Z'/></System><EventData><Data>Three_Kingdoms.exe</Data><Data>1.7.6.0</Data><Data>69ce4df8</Data><Data>Three_Kingdoms.exe</Data><Data>1.7.6.0</Data><Data>69ce4df8</Data><Data>c0000005</Data><Data>000000000055b7e0</Data></EventData></Event>";

    fn utc(y: i64, mo: u32, d: u32, h: u64, mi: u64, s: u64) -> u64 {
        crate::fmt::days_from_civil(y, mo, d) + h * 3600 + mi * 60 + s
    }

    #[test]
    fn crash_events_parse_utc_and_skip_other_programs() {
        let ev = parse_crash_events(EVENT_XML);
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].when, utc(2026, 11, 8, 4, 3, 9));
        assert_eq!(ev[0].location(), "Three_Kingdoms.exe+0x2326aa4");
        assert_eq!(ev[0].code, "c0000005");
        assert_eq!(ev[1].location(), "Three_Kingdoms.exe+0x55b7e0");
        let text = render_crash_events(&Ok(ev));
        assert!(text.contains("- Date: 2026-11-08 04:03 UTC"));
        assert!(text.contains("time stamp: 0x69ce4df8"));
        assert!(text.contains("Fault offset: 0000000002326aa4 (Three_Kingdoms.exe+0x2326aa4)"));
    }

    // The real case (thread 1568838476181545013): court-tab crash at 03:36 wrote a dump at 03:36:41;
    // the game was launched again at 03:50; the battle crash at 04:03:09 wrote no dump.
    #[test]
    fn an_old_dump_is_not_matched_to_a_newer_crash() {
        let dump = utc(2026, 11, 8, 3, 36, 41);
        let launch = Some(utc(2026, 11, 8, 3, 50, 0));
        assert!(matches!(match_dump(dump, Some(utc(2026, 11, 8, 4, 3, 9)), launch), DumpMatch::Earlier(_)));
        // The same dump for the court-tab report (event 16 s after the dump, launch at 03:22 before it).
        assert_eq!(match_dump(dump, Some(utc(2026, 11, 8, 3, 36, 57)), Some(utc(2026, 11, 7, 23, 22, 0))), DumpMatch::Matches);
    }

    #[test]
    fn dump_match_falls_back_to_the_last_launch() {
        let l = 1_000_000;
        assert_eq!(match_dump(l + 500, None, Some(l)), DumpMatch::Matches);
        assert!(matches!(match_dump(l - 500, None, Some(l)), DumpMatch::Earlier(_)));
        assert_eq!(match_dump(l, None, None), DumpMatch::Unverified);
        // A dump well after the newest Windows event (Windows missed that crash) during the last launch.
        assert_eq!(match_dump(l + 5_000, Some(l + 100), Some(l)), DumpMatch::Matches);
    }

    #[test]
    fn crash_lines_name_the_crash_and_the_left_out_dump() {
        let ev = parse_crash_events(EVENT_XML);
        let t = crash_lines(ev.first(), Some(("D2026-11-07_T22-36-41.mdmp", "2026-11-08 03:36 UTC", "Windows recorded a newer crash at 2026-11-08 04:03 UTC")), true);
        assert!(t.contains("**Latest crash (Windows):** Three_Kingdoms.exe+0x2326aa4 · c0000005 · 2026-11-08 04:03 UTC"));
        assert!(t.contains("No crash dump for this crash: the newest dump (D2026-11-07_T22-36-41.mdmp, 2026-11-08 03:36 UTC) is from an earlier crash"));
        assert!(t.contains("se_crash.txt"));
        assert_eq!(crash_lines(None, None, false), "");
    }

    #[test]
    fn se_crash_file_counts_only_since_the_last_launch() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("se_crash.txt"), "fault").unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let cc = |l: Option<u64>| CrashContext { events: Ok(vec![]), last_launch: l };
        assert!(se_crash_file(Some(dir.path()), &cc(Some(now - 100)), now).is_some());
        assert!(se_crash_file(Some(dir.path()), &cc(Some(now + 3_600)), now).is_none());
        assert!(se_crash_file(Some(dir.path()), &cc(None), now).is_some());
        assert!(se_crash_file(None, &cc(None), now).is_none());
    }

}
