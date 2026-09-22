//! Installs from Nexus Mods (nxm:// links and archives on disk), switching between installed
//! Nexus files, and "Force update" of Workshop items through the Steam helper process.

use crate::app::{App, DIRTY_ALL, DIRTY_DETAILS, DIRTY_LIST};
use crate::events::UiEvent;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tkmm_core::nexus::{self, InstallError, InstallRequest, Installed};
use tkmm_core::steam_ugc::{self, Event};

impl App {
    pub unsafe fn set_task(&self, text: &str) {
        self.task_label.set_text(&qt_core::qs(text));
        self.task_label.set_visible(!text.is_empty());
        self.task_expires.set(None);
    }

    /// A result line that goes away by itself after a few seconds.
    pub unsafe fn set_task_done(&self, text: &str) {
        self.set_task(text);
        self.task_expires.set(Some(std::time::Instant::now() + std::time::Duration::from_secs(10)));
    }

    /// A "Mod Manager Download" link from nexusmods.com.
    pub unsafe fn install_nxm(self: &Rc<Self>, url: &str) {
        let link = match nexus::parse_nxm(url) {
            Ok(l) => l,
            Err(e) => return crate::dialogs::error(&self.window, &format!("Cannot install from Nexus Mods: {e}")),
        };
        let key = self.ctx.settings().nexus_api_key;
        if key.trim().is_empty() {
            crate::dialogs::error(
                &self.window,
                "Add your Nexus Mods API key first (Settings > Nexus Mods), then click \"Mod Manager Download\" again.",
            );
            self.set_page(crate::app::PAGE_SETTINGS);
            return;
        }
        self.set_task(&format!("Nexus Mods: fetching mod {} file {}…", link.mod_id, link.file_id));
        self.bus.spawn(move |bus| {
            let b = bus.clone();
            let progress = move |f: f64| b.send(UiEvent::Task(format!("Nexus Mods: downloading… {}%", (f * 100.0).round())));
            match nexus::download(&key, &link, &progress) {
                Ok(d) => {
                    bus.send(UiEvent::Task(format!("Nexus Mods: installing {}…", d.request.mod_name)));
                    let result = nexus::install_archive(&nexus::root(), &d.archive, &d.request, None);
                    Some(UiEvent::NexusInstalled { result, archive: d.archive, request: d.request, downloaded: true })
                }
                Err(e) => Some(UiEvent::NexusFailed(e)),
            }
        });
    }

    /// "Install from archive…": a zip/7z/rar picked from disk (or dropped on the window).
    pub unsafe fn install_archive_file(self: &Rc<Self>, path: &Path) {
        let request = nexus::request_for_local(path);
        self.run_install(path.to_path_buf(), request, None, false);
    }

    pub unsafe fn pick_archive(self: &Rc<Self>) {
        let file = qt_widgets::QFileDialog::get_open_file_name_4a(
            &self.window,
            &qt_core::qs("Install a mod archive"),
            &qt_core::qs(""),
            &qt_core::qs("Mod archives (*.zip *.7z *.rar);;All files (*.*)"),
        )
        .to_std_string();
        if !file.is_empty() {
            self.install_archive_file(Path::new(&file));
        }
    }

    unsafe fn run_install(self: &Rc<Self>, archive: PathBuf, request: InstallRequest, variant: Option<String>, downloaded: bool) {
        self.set_task(&format!("Installing {}…", if request.mod_name.is_empty() { &request.slot } else { &request.mod_name }));
        self.bus.spawn(move |_| {
            let result = nexus::install_archive(&nexus::root(), &archive, &request, variant.as_deref());
            Some(UiEvent::NexusInstalled { result, archive, request, downloaded })
        });
    }

    pub unsafe fn on_nexus_installed(self: &Rc<Self>, result: Result<Installed, InstallError>, archive: PathBuf, request: InstallRequest, downloaded: bool) {
        match result {
            Ok(done) => {
                if downloaded {
                    let _ = std::fs::remove_file(&archive);
                }
                // Enable packs that are new to the list; a re-install or another version of an
                // installed mod keeps the state the profile already has.
                let keys: Vec<String> = {
                    let st = self.st.borrow();
                    done.packs.iter().map(|p| format!("nx:{}/{}", done.slot, p)).filter(|k| !st.by_key.contains_key(k)).collect()
                };
                self.pending_enable.borrow_mut().extend(keys);
                let clash = self.name_clashes(&done);
                let name = if done.mod_name.is_empty() { done.slot.clone() } else { done.mod_name.clone() };
                let version = if request.file.version.is_empty() { String::new() } else { format!(" {}", request.file.version) };
                self.set_task_done(&format!("Installed {name}{version} ({} pack{}).", done.packs.len(), if done.packs.len() == 1 { "" } else { "s" }));
                if !clash.is_empty() {
                    crate::dialogs::info(
                        &self.window,
                        "Same pack name twice",
                        &format!(
                            "{name} installs packs that are also installed from somewhere else:\n\n{}\n\nThe game loads packs by file name, so only enable one copy of each.",
                            clash.join("\n")
                        ),
                    );
                }
                self.rescan();
            }
            Err(InstallError::Variants(folders)) => {
                self.set_task("");
                let chosen = crate::dialogs::choose(
                    &self.window,
                    "Choose a variant",
                    "This archive ships the same pack in several folders (alternative versions).\nWhich one do you want?",
                    &folders,
                );
                match chosen {
                    Some(v) => self.run_install(archive, request, Some(v), downloaded),
                    None if downloaded => {
                        let _ = std::fs::remove_file(&archive);
                    }
                    None => {}
                }
            }
            Err(InstallError::Failed(e)) => {
                if downloaded {
                    let _ = std::fs::remove_file(&archive);
                }
                self.on_nexus_failed(&e);
            }
        }
    }

    pub unsafe fn on_nexus_failed(self: &Rc<Self>, e: &str) {
        self.set_task("");
        crate::dialogs::error(&self.window, &format!("Install failed: {e}"));
    }

    /// Installed packs from other sources that share a file name with this install.
    fn name_clashes(&self, done: &Installed) -> Vec<String> {
        let st = self.st.borrow();
        let prefix = format!("nx:{}/", done.slot);
        let mut out = Vec::new();
        for p in &done.packs {
            for m in st.mods.iter().filter(|m| m.file.eq_ignore_ascii_case(p) && !m.key.starts_with(&prefix)) {
                out.push(format!("{} ({})", m.file, st.source_label(m)));
            }
        }
        out
    }

    /// Called after every scan: switch on what an install just added.
    pub fn enable_pending(self: &Rc<Self>) {
        let keys: Vec<String> = {
            let st = self.st.borrow();
            self.pending_enable.borrow().iter().filter(|k| st.by_key.contains_key(*k)).cloned().collect()
        };
        if keys.is_empty() {
            return;
        }
        self.pending_enable.borrow_mut().retain(|k| !keys.contains(k));
        if self.st.borrow().game_running {
            return;
        }
        self.st.borrow_mut().update_active(|p| tkmm_core::profile_ops::toggle(&mut p.entries, &keys, Some(true)));
        self.mark(DIRTY_ALL);
    }

    pub unsafe fn nexus_set_active(self: &Rc<Self>, slot: &str, dir: &str) {
        if self.st.borrow().game_running {
            return crate::dialogs::error(&self.window, "Quit the game before switching versions.");
        }
        match nexus::set_active(&nexus::root(), slot, dir) {
            Ok(()) => self.rescan(),
            Err(e) => crate::dialogs::error(&self.window, &e),
        }
    }

    pub unsafe fn nexus_remove(self: &Rc<Self>, slot: &str, dir: &str, label: &str) {
        if self.st.borrow().game_running {
            return crate::dialogs::error(&self.window, "Quit the game before deleting mod files.");
        }
        if !crate::dialogs::confirm(&self.window, "Delete installed version", &format!("Delete {label} from the manager's Nexus folder?")) {
            return;
        }
        match nexus::remove_version(&nexus::root(), slot, dir) {
            Ok(()) => self.rescan(),
            Err(e) => crate::dialogs::error(&self.window, &e),
        }
    }

    /// Refresh the Nexus file lists of installed mods. `verbose` = the user asked (report the
    /// result); otherwise it is the quiet startup check, which honours the cache age.
    pub fn check_nexus_updates(self: &Rc<Self>, verbose: bool) {
        let s = self.ctx.settings();
        if s.nexus_api_key.trim().is_empty() || nexus::slots(&nexus::root()).values().all(|i| i.mod_id.is_none()) {
            if verbose {
                unsafe { crate::dialogs::info(&self.window, "Nexus Mods", "No Nexus mods to check (or no API key set).") };
            }
            return;
        }
        let max_age = if verbose { 0 } else { u64::from(s.workshop_cache_hours.max(1)) * 3600 };
        let key = s.nexus_api_key;
        self.bus.spawn(move |_| Some(UiEvent::NexusChecked(nexus::check_updates(&key, max_age), verbose)));
    }

    pub unsafe fn on_nexus_checked(self: &Rc<Self>, r: Result<usize, String>, verbose: bool) {
        match r {
            Ok(n) => {
                if verbose {
                    let text = match n {
                        0 => "Every installed Nexus mod is up to date.".to_string(),
                        1 => "1 Nexus mod has a newer file (red dot in the list).".to_string(),
                        n => format!("{n} Nexus mods have newer files (red dots in the list)."),
                    };
                    crate::dialogs::info(&self.window, "Nexus Mods", &text);
                }
                if n > 0 || verbose {
                    self.rescan();
                }
            }
            Err(e) if verbose => crate::dialogs::error(&self.window, &format!("Nexus update check failed: {e}")),
            Err(e) => eprintln!("nexus update check: {e}"),
        }
    }

    // ------------------------------------------------------------------ Steam

    /// Ask Steam to download the newest version of these Workshop items now.
    pub unsafe fn force_update(self: &Rc<Self>, ids: Vec<String>) {
        if self.steam_busy.get() {
            return;
        }
        if self.st.borrow().game_running {
            return crate::dialogs::error(&self.window, "Quit the game first: Steam does not update Workshop items of a running game.");
        }
        let ids: Vec<u64> = ids.iter().filter_map(|s| s.parse().ok()).collect();
        if ids.is_empty() {
            return;
        }
        self.steam_busy.set(true);
        self.steam_results.borrow_mut().clear();
        self.set_task(&format!("Steam: updating {} Workshop item{}…", ids.len(), if ids.len() == 1 { "" } else { "s" }));
        self.bus.spawn(move |bus| {
            let b = bus.clone();
            let r = steam_ugc::spawn_helper(&ids, &mut |e| b.send(UiEvent::SteamDl(e)));
            Some(UiEvent::SteamDlDone(r))
        });
    }

    pub unsafe fn on_steam_event(self: &Rc<Self>, e: Event) {
        match e {
            Event::Progress { id, done, total } => {
                let title = self.st.borrow().workshop.get(&id.to_string()).map(|w| w.title.clone()).unwrap_or_else(|| id.to_string());
                let pct = if total > 0 { done * 100 / total } else { 0 };
                self.set_task(&format!("Steam: downloading {title}… {pct}%"));
            }
            Event::Finished { id, ok, message } => self.steam_results.borrow_mut().push((id, ok, message)),
            Event::Fatal { .. } => {}
        }
    }

    pub unsafe fn on_steam_done(self: &Rc<Self>, r: Result<(), String>) {
        self.steam_busy.set(false);
        self.rescan();
        let results = std::mem::take(&mut *self.steam_results.borrow_mut());
        let failed: Vec<String> = {
            let st = self.st.borrow();
            results
                .iter()
                .filter(|(_, ok, _)| !ok)
                .map(|(id, _, msg)| {
                    let title = st.workshop.get(&id.to_string()).map(|w| w.title.clone()).unwrap_or_else(|| id.to_string());
                    format!("{title}: {msg}")
                })
                .collect()
        };
        let ok = results.iter().filter(|r| r.1).count();
        match r {
            Err(e) => {
                self.set_task("");
                crate::dialogs::error(&self.window, &format!("Force update failed: {e}"));
            }
            Ok(()) if failed.is_empty() => self.set_task_done(&format!("Steam: {ok} Workshop item{} up to date.", if ok == 1 { " is" } else { "s are" })),
            Ok(()) => {
                self.set_task("");
                crate::dialogs::error(&self.window, &format!("{ok} updated, {} failed:\n\n{}", failed.len(), failed.join("\n")));
            }
        }
        self.mark(DIRTY_LIST | DIRTY_DETAILS);
    }

    /// Workshop ids of every mod with a pending update.
    pub fn pending_workshop_ids(&self) -> Vec<String> {
        let st = self.st.borrow();
        let cutoff = st.cutoff(self.ctx.settings().outdated_before);
        let mut ids: Vec<String> = st
            .mods
            .iter()
            .filter(|m| tkmm_core::status::mod_status(Some(m), st.ws_of(m), cutoff).kind == tkmm_core::status::StatusKind::Pending)
            .filter_map(|m| m.workshop_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }
}

/// Crash log written by the panic hook (see main.rs).
pub fn crash_log_path() -> PathBuf {
    tkmm_core::paths::app_data_dir().join("crash.log")
}
