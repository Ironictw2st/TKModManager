//! The Settings page (opened with the Settings button in the header).

use crate::app::{App, DIRTY_ALL, DIRTY_LIST, PAGE_LOGS};
use crate::details::{bold, colored, label};
use crate::events::{HashPurpose, UiEvent};
use crate::icons;
use qt_core::{qs, QBox, QDate, QListOfQString, QVariant, SlotNoArgs, SlotOfBool, SlotOfInt};
use qt_widgets::{
    q_header_view::ResizeMode, QCheckBox, QComboBox, QDateEdit, QFileDialog, QGridLayout, QGroupBox, QHBoxLayout, QLabel, QLineEdit, QListWidget,
    QPlainTextEdit, QProgressBar, QPushButton, QScrollArea, QSpinBox, QTreeWidget, QTreeWidgetItem, QVBoxLayout, QWidget,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tkmm_core::dll::{self, DllConfig, InstalledDll, RemoteDll};
use tkmm_core::fmt::{days_from_civil, format_bytes, ymd};
use tkmm_core::hash::{PackHash, Progress};
use tkmm_core::sync::{self, SyncStatus};
use tkmm_core::workshop::CollectionResult;
use tkmm_core::{ops, paths, profile_ops, workshop};

pub struct SettingsUi {
    pub root: QBox<QScrollArea>,
    // game
    game_root: QBox<QLabel>,
    game_change: QBox<QPushButton>,
    game_auto: QBox<QPushButton>,
    ws_dir: QBox<QLabel>,
    ws_change: QBox<QPushButton>,
    ws_auto: QBox<QPushButton>,
    version: QBox<QLabel>,
    open_game: QBox<QPushButton>,
    refresh_mods: QBox<QPushButton>,
    folders: QBox<QListWidget>,
    folder_add: QBox<QPushButton>,
    folder_remove: QBox<QPushButton>,
    cutoff: QBox<QDateEdit>,
    cutoff_reset: QBox<QPushButton>,
    // behaviour
    tray: QBox<QCheckBox>,
    app_updates: QBox<QCheckBox>,
    dll_updates: QBox<QCheckBox>,
    cache_hours: QBox<QSpinBox>,
    // script extender
    dll_game: QBox<QLabel>,
    dll_list: QBox<QTreeWidget>,
    dll_folder: QBox<QPushButton>,
    dll_remove: QBox<QPushButton>,
    dll_check: QBox<QPushButton>,
    dll_channel: QBox<QComboBox>,
    dll_import: QBox<QPushButton>,
    dll_log: QBox<QPushButton>,
    dll_latest: QBox<QLabel>,
    dll_install: QBox<QPushButton>,
    dll_progress: QBox<QProgressBar>,
    dll_auto: QBox<QCheckBox>,
    cfg_build: QBox<QLineEdit>,
    cfg_short: QBox<QLineEdit>,
    cfg_modified: QBox<QComboBox>,
    cfg_save: QBox<QPushButton>,
    // mod list preview
    preview: QBox<QPlainTextEdit>,
    // profiles
    imp_ca: QBox<QPushButton>,
    imp_collection: QBox<QPushButton>,
    imp_text: QBox<QPushButton>,
    export: QBox<QPushButton>,
    verify: QBox<QPushButton>,
    profile_msg: QBox<QLabel>,
    // about
    about: QBox<QLabel>,
    open_data: QBox<QPushButton>,
    check_app: QBox<QPushButton>,

    remote: RefCell<Option<RemoteDll>>,
}

unsafe fn row(grid: &QBox<QGridLayout>, r: i32, title: &str) {
    let l = QLabel::from_q_string(&qs(title));
    grid.add_widget_3a(&l, r, 0);
}

unsafe fn button(text: &str) -> QBox<QPushButton> {
    QPushButton::from_q_string(&qs(text))
}

impl SettingsUi {
    pub fn build() -> SettingsUi {
        unsafe {
            let root = QScrollArea::new_0a();
            root.set_widget_resizable(true);
            root.set_frame_shape(qt_widgets::q_frame::Shape::NoFrame);
            let page = QWidget::new_0a();
            let v = QVBoxLayout::new_1a(&page);
            v.set_contents_margins_4a(0, 0, 8, 0);
            v.set_spacing(10);

            // Game and mod folders.
            let g = QGroupBox::from_q_string(&qs("Game and mod folders"));
            let grid = QGridLayout::new_1a(&g);
            grid.set_column_stretch(1, 1);
            row(&grid, 0, "Game folder");
            let game_root = label("");
            grid.add_widget_3a(&game_root, 0, 1);
            let game_change = button("Change…");
            let game_auto = button("Auto-detect");
            grid.add_widget_3a(&game_change, 0, 2);
            grid.add_widget_3a(&game_auto, 0, 3);
            row(&grid, 1, "Workshop folder");
            let ws_dir = label("");
            grid.add_widget_3a(&ws_dir, 1, 1);
            let ws_change = button("Change…");
            let ws_auto = button("Auto-detect");
            grid.add_widget_3a(&ws_change, 1, 2);
            grid.add_widget_3a(&ws_auto, 1, 3);
            row(&grid, 2, "Game version");
            let version = label("");
            grid.add_widget_3a(&version, 2, 1);
            let open_game = button("Open folder");
            let refresh_mods = button("Refresh mods");
            grid.add_widget_3a(&open_game, 2, 2);
            grid.add_widget_3a(&refresh_mods, 2, 3);
            row(&grid, 3, "Extra mod folders");
            let folders = QListWidget::new_0a();
            folders.set_maximum_height(90);
            grid.add_widget_3a(&folders, 3, 1);
            let folder_add = button("Add folder…");
            let folder_remove = button("Remove");
            grid.add_widget_3a(&folder_add, 3, 2);
            grid.add_widget_3a(&folder_remove, 3, 3);
            let hint = label("Packs in extra folders (for example your RPFM MyMods folder) load straight from there; nothing is copied into data/. The mod list is written to tkmm_mods.txt in the game folder; CA's used_mods.txt is only read.");
            grid.add_widget_5a(&hint, 4, 1, 1, 3);
            row(&grid, 5, "Older than patch");
            let cutoff = QDateEdit::new();
            cutoff.set_calendar_popup(true);
            cutoff.set_display_format(&qs("d MMM yyyy"));
            cutoff.set_tool_tip(&qs("Workshop mods last updated before this date get the amber dot"));
            grid.add_widget_3a(&cutoff, 5, 1);
            let cutoff_reset = button("Use game build date");
            grid.add_widget_3a(&cutoff_reset, 5, 2);
            v.add_widget(&g);

            // Behaviour.
            let g = QGroupBox::from_q_string(&qs("Behaviour"));
            let b = QVBoxLayout::new_1a(&g);
            let tray = QCheckBox::from_q_string(&qs("Keep running in the tray when the window is closed (tray menu: Play, profiles, Quit)"));
            let app_updates = QCheckBox::from_q_string(&qs("Check for app updates on startup"));
            let dll_updates = QCheckBox::from_q_string(&qs("Check for script-extender updates on startup"));
            b.add_widget(&tray);
            b.add_widget(&app_updates);
            b.add_widget(&dll_updates);
            let hr = QHBoxLayout::new_0a();
            b.add_layout_1a(&hr);
            hr.add_widget(QLabel::from_q_string(&qs("Re-fetch Workshop titles and dates after")).into_ptr());
            let cache_hours = QSpinBox::new_0a();
            cache_hours.set_range(1, 24 * 14);
            cache_hours.set_suffix(&qs(" hours"));
            hr.add_widget(&cache_hours);
            hr.add_stretch_1a(1);
            b.add_widget(&label("The look follows the Windows light / dark setting."));
            v.add_widget(&g);

            // Script extender.
            let g = QGroupBox::from_q_string(&qs("Script extender (DLL)"));
            let s = QVBoxLayout::new_1a(&g);
            let dll_game = label("");
            s.add_widget(&dll_game);
            let dll_list = QTreeWidget::new_0a();
            let h = QListOfQString::new_0a();
            for t in ["Version", "Game build", "Status"] {
                h.append_q_string(&qs(t));
            }
            dll_list.set_header_labels(&h);
            dll_list.set_root_is_decorated(false);
            dll_list.set_maximum_height(120);
            dll_list.header().set_section_resize_mode_2a(2, ResizeMode::Stretch);
            s.add_widget(&dll_list);
            let r1 = QHBoxLayout::new_0a();
            s.add_layout_1a(&r1);
            let dll_folder = button("Open folder");
            let dll_remove = button("Remove version");
            let dll_log = button("View log");
            let dll_import = button("Import local DLL…");
            for w in [&dll_folder, &dll_remove, &dll_log, &dll_import] {
                r1.add_widget(w);
            }
            r1.add_stretch_1a(1);
            let r2 = QHBoxLayout::new_0a();
            s.add_layout_1a(&r2);
            let dll_check = button("Check for DLL updates");
            let dll_channel = QComboBox::new_0a();
            dll_channel.add_item_q_string_q_variant(&qs("Stable releases"), &QVariant::from_q_string(&qs("stable")));
            dll_channel.add_item_q_string_q_variant(&qs("Include pre-releases"), &QVariant::from_q_string(&qs("prerelease")));
            let dll_latest = QLabel::new();
            let dll_install = button("Install");
            dll_install.set_visible(false);
            r2.add_widget(&dll_check);
            r2.add_widget(&dll_channel);
            r2.add_widget_2a(&dll_latest, 1);
            r2.add_widget(&dll_install);
            let dll_progress = QProgressBar::new_0a();
            dll_progress.set_range(0, 100);
            dll_progress.set_visible(false);
            s.add_widget(&dll_progress);
            let dll_auto = QCheckBox::from_q_string(&qs("Also inject when the game was started outside the manager (while this app is running)"));
            s.add_widget(&dll_auto);
            let cfg_title = label("Main-menu build text (script_extender.cfg)");
            bold(&cfg_title);
            s.add_widget(&cfg_title);
            let cg = QGridLayout::new_0a();
            s.add_layout_1a(&cg);
            cg.set_column_stretch(1, 1);
            cg.add_widget_3a(QLabel::from_q_string(&qs("Build number")).into_ptr(), 0, 0);
            let cfg_build = QLineEdit::new();
            cfg_build.set_placeholder_text(&qs("leave empty to keep the game's own text"));
            cg.add_widget_3a(&cfg_build, 0, 1);
            cg.add_widget_3a(QLabel::from_q_string(&qs("Short form")).into_ptr(), 1, 0);
            let cfg_short = QLineEdit::new();
            cg.add_widget_3a(&cfg_short, 1, 1);
            cg.add_widget_3a(QLabel::from_q_string(&qs("\"Modified\" flag")).into_ptr(), 2, 0);
            let cfg_modified = QComboBox::new_0a();
            for (t, val) in [("Leave alone", ""), ("Show as unmodified", "0"), ("Show as modified", "1")] {
                cfg_modified.add_item_q_string_q_variant(&qs(t), &QVariant::from_q_string(&qs(val)));
            }
            cg.add_widget_3a(&cfg_modified, 2, 1);
            let cfg_save = button("Save");
            cg.add_widget_3a(&cfg_save, 2, 2);
            v.add_widget(&g);

            // Generated mod list.
            let g = QGroupBox::from_q_string(&qs("Generated mod list"));
            let p = QVBoxLayout::new_1a(&g);
            p.add_widget(&label("Exactly what is written to tkmm_mods.txt at launch for the active profile (before the optional skip-intro pack)."));
            let preview = QPlainTextEdit::new();
            preview.set_read_only(true);
            preview.set_minimum_height(160);
            let f = qt_gui::QFont::new_copy(preview.font());
            f.set_family(&qs("Consolas"));
            preview.set_font(&f);
            p.add_widget(&preview);
            v.add_widget(&g);

            // Profiles.
            let g = QGroupBox::from_q_string(&qs("Profiles: import, export and co-op sync check"));
            let p = QVBoxLayout::new_1a(&g);
            let r = QHBoxLayout::new_0a();
            let imp_ca = button("Import CA launcher list…");
            let imp_collection = button("Import Workshop collection…");
            let imp_text = button("Import from text…");
            let export = button("Export active profile (with hashes)");
            let verify = button("Verify against an export…");
            for w in [&imp_ca, &imp_collection, &imp_text, &export, &verify] {
                r.add_widget(w);
            }
            r.add_stretch_1a(1);
            p.add_layout_1a(&r);
            let profile_msg = label("");
            p.add_widget(&profile_msg);
            v.add_widget(&g);

            // About.
            let g = QGroupBox::from_q_string(&qs("About"));
            let a = QHBoxLayout::new_1a(&g);
            let about = label("");
            a.add_widget_2a(&about, 1);
            let open_data = button("Open data folder");
            let check_app = button("Check for app updates");
            a.add_widget(&open_data);
            a.add_widget(&check_app);
            v.add_widget(&g);
            v.add_stretch_1a(1);
            root.set_widget(&page);

            SettingsUi {
                root,
                game_root,
                game_change,
                game_auto,
                ws_dir,
                ws_change,
                ws_auto,
                version,
                open_game,
                refresh_mods,
                folders,
                folder_add,
                folder_remove,
                cutoff,
                cutoff_reset,
                tray,
                app_updates,
                dll_updates,
                cache_hours,
                dll_game,
                dll_list,
                dll_folder,
                dll_remove,
                dll_check,
                dll_channel,
                dll_import,
                dll_log,
                dll_latest,
                dll_install,
                dll_progress,
                dll_auto,
                cfg_build,
                cfg_short,
                cfg_modified,
                cfg_save,
                preview,
                imp_ca,
                imp_collection,
                imp_text,
                export,
                verify,
                profile_msg,
                about,
                open_data,
                check_app,
                remote: RefCell::new(None),
            }
        }
    }

    /// Change settings through the context (persists) and redraw.
    unsafe fn update_settings(app: &Rc<App>, f: impl FnOnce(&mut tkmm_core::settings::Settings)) {
        let mut s = app.ctx.settings();
        let before = (s.game_root.clone(), s.workshop_dir.clone(), s.extra_mod_dirs.clone());
        f(&mut s);
        let changed_paths = before != (s.game_root.clone(), s.workshop_dir.clone(), s.extra_mod_dirs.clone());
        let tray = s.minimize_to_tray;
        if let Err(e) = app.ctx.set_settings(s) {
            crate::dialogs::error(&app.window, &e);
        }
        app.tray.set_visible(tray);
        if changed_paths {
            app.rescan();
            app.refresh_dll();
        }
        app.mark(DIRTY_ALL);
    }

    pub unsafe fn connect(&self, app: &Rc<App>) {
        let w = &app.window;
        macro_rules! on_click {
            ($btn:expr, $body:expr) => {{
                let this = app.clone();
                let f = $body;
                $btn.clicked().connect(&SlotNoArgs::new(w, move || f(&this)));
            }};
        }
        on_click!(self.game_change, |a: &Rc<App>| {
            let d = QFileDialog::get_existing_directory_2a(&a.window, &qs("Select the Total War THREE KINGDOMS folder")).to_std_string();
            if !d.is_empty() {
                SettingsUi::update_settings(a, |s| s.game_root = Some(d));
            }
        });
        on_click!(self.game_auto, |a: &Rc<App>| SettingsUi::update_settings(a, |s| s.game_root = None));
        on_click!(self.ws_change, |a: &Rc<App>| {
            let d = QFileDialog::get_existing_directory_2a(&a.window, &qs("Select the Workshop content folder (779340)")).to_std_string();
            if !d.is_empty() {
                SettingsUi::update_settings(a, |s| s.workshop_dir = Some(d));
            }
        });
        on_click!(self.ws_auto, |a: &Rc<App>| SettingsUi::update_settings(a, |s| s.workshop_dir = None));
        on_click!(self.open_game, |a: &Rc<App>| {
            if let Some(r) = a.st.borrow().paths.game_root.clone() {
                crate::details::open_url(&format!("file:///{}", r.replace('\\', "/")));
            }
        });
        on_click!(self.refresh_mods, |a: &Rc<App>| a.rescan());
        on_click!(self.folder_add, |a: &Rc<App>| {
            let d = QFileDialog::get_existing_directory_2a(&a.window, &qs("Select a folder that holds .pack files")).to_std_string();
            if !d.is_empty() {
                SettingsUi::update_settings(a, |s| {
                    if !s.extra_mod_dirs.iter().any(|x| x.eq_ignore_ascii_case(&d)) {
                        s.extra_mod_dirs.push(d.replace('/', "\\"));
                    }
                });
            }
        });
        on_click!(self.folder_remove, |a: &Rc<App>| {
            let cur = a.settings.folders.current_item();
            if cur.is_null() {
                return;
            }
            let d = cur.text().to_std_string();
            SettingsUi::update_settings(a, |s| s.extra_mod_dirs.retain(|x| x != &d));
        });
        let this = app.clone();
        self.cutoff.editing_finished().connect(&SlotNoArgs::new(w, move || {
            if this.suppress.get() {
                return;
            }
            let d = this.settings.cutoff.date();
            let t = days_from_civil(d.year_0a() as i64, d.month_0a() as u32, d.day_0a() as u32);
            SettingsUi::update_settings(&this, |s| s.outdated_before = Some(t));
        }));
        on_click!(self.cutoff_reset, |a: &Rc<App>| SettingsUi::update_settings(a, |s| s.outdated_before = None));

        for (cb, which) in [(&self.tray, 0), (&self.app_updates, 1), (&self.dll_updates, 2), (&self.dll_auto, 3)] {
            let this = app.clone();
            cb.clicked().connect(&SlotOfBool::new(w, move |on| {
                SettingsUi::update_settings(&this, |s| match which {
                    0 => s.minimize_to_tray = on,
                    1 => s.check_app_updates = on,
                    2 => s.check_dll_updates = on,
                    _ => s.auto_inject_external = on,
                })
            }));
        }
        let this = app.clone();
        self.cache_hours.value_changed().connect(&SlotOfInt::new(w, move |h| {
            if !this.suppress.get() {
                SettingsUi::update_settings(&this, |s| s.workshop_cache_hours = h.max(1) as u32);
            }
        }));

        on_click!(self.dll_folder, |a: &Rc<App>| {
            if let Some(d) = a.settings.selected_dll(a) {
                crate::details::reveal(&d.path);
            }
        });
        on_click!(self.dll_remove, |a: &Rc<App>| {
            let Some(d) = a.settings.selected_dll(a) else { return };
            if !crate::dialogs::confirm(&a.window, "Remove DLL", &format!("Remove script extender {}?", d.version)) {
                return;
            }
            match dll::dll_remove(d.version.clone()) {
                Ok(()) => a.refresh_dll(),
                Err(e) => crate::dialogs::error(&a.window, &e),
            }
        });
        on_click!(self.dll_log, |a: &Rc<App>| a.set_page(PAGE_LOGS));
        on_click!(self.dll_import, |a: &Rc<App>| {
            let file = QFileDialog::get_open_file_name_4a(&a.window, &qs("Select script_extender.dll"), &qs(""), &qs("DLL (*.dll)")).to_std_string();
            if file.is_empty() {
                return;
            }
            let Some(version) = crate::dialogs::ask_text(&a.window, "Import DLL", "Version of this DLL (semver, e.g. 0.28.0):", "") else { return };
            match dll::dll_import_local(&a.ctx, &file, &version) {
                Ok(d) => {
                    a.refresh_dll();
                    crate::dialogs::info(&a.window, "Imported", &format!("Imported {}; its manifest is pinned to the current game build.", d.version));
                }
                Err(e) => crate::dialogs::error(&a.window, &e),
            }
        });
        on_click!(self.dll_check, |a: &Rc<App>| a.settings.check_dll(a));
        let this = app.clone();
        self.dll_channel.activated().connect(&SlotOfInt::new(w, move |_| {
            let v = this.settings.dll_channel.current_data_0a().to_string().to_std_string();
            SettingsUi::update_settings(&this, |s| s.dll_channel = v);
            this.settings.check_dll(&this);
        }));
        on_click!(self.dll_install, |a: &Rc<App>| {
            let Some(r) = a.settings.remote.borrow().clone() else { return };
            a.settings.dll_install.set_enabled(false);
            a.settings.dll_progress.set_value(0);
            a.settings.dll_progress.set_visible(true);
            let ctx = a.ctx.clone();
            a.bus.spawn(move |bus| {
                let b = bus.clone();
                Some(UiEvent::DllInstalled(dll::dll_install(&ctx, &r, &move |f| b.send(UiEvent::DllProgress(f)))))
            });
        });
        on_click!(self.cfg_save, |a: &Rc<App>| {
            let s = &a.settings;
            let m = s.cfg_modified.current_data_0a().to_string().to_std_string();
            let cfg = DllConfig {
                build_number: s.cfg_build.text().to_std_string(),
                build_number_short: s.cfg_short.text().to_std_string(),
                build_modified: match m.as_str() {
                    "0" => Some(false),
                    "1" => Some(true),
                    _ => None,
                },
            };
            match dll::dll_write_cfg(cfg) {
                Ok(()) => crate::dialogs::info(&a.window, "Saved", "Saved. It applies at the next injection."),
                Err(e) => crate::dialogs::error(&a.window, &e),
            }
        });

        on_click!(self.imp_ca, |a: &Rc<App>| a.settings.import_ca(a));
        on_click!(self.imp_collection, |a: &Rc<App>| {
            let Some(input) = crate::dialogs::ask_text(&a.window, "Import Workshop collection", "Collection link or id:", "") else { return };
            a.settings.profile_msg.set_text(&qs("Fetching the collection from Steam…"));
            a.bus.spawn(move |_| Some(UiEvent::Collection(workshop::workshop_collection(&input))));
        });
        on_click!(self.imp_text, |a: &Rc<App>| a.settings.import_text(a));
        on_click!(self.export, |a: &Rc<App>| {
            a.settings.profile_msg.set_text(&qs("Hashing the enabled packs…"));
            a.hash_enabled(HashPurpose::Export);
        });
        on_click!(self.verify, |a: &Rc<App>| {
            let Some(text) = crate::dialogs::ask_multiline(&a.window, "Verify against an export", "Paste your partner's export. The enabled packs of the active profile are compared with it (file size and sha256).", "Compare")
            else {
                return;
            };
            *a.verify_text.borrow_mut() = text;
            a.settings.profile_msg.set_text(&qs("Hashing the enabled packs…"));
            a.hash_enabled(HashPurpose::Verify);
        });

        on_click!(self.open_data, |a: &Rc<App>| {
            let _ = a;
            crate::details::open_url(&format!("file:///{}", paths::app_data_dir().to_string_lossy().replace('\\', "/")));
        });
        on_click!(self.check_app, |a: &Rc<App>| a.check_app_update(true));
    }

    unsafe fn selected_dll(&self, app: &Rc<App>) -> Option<InstalledDll> {
        let cur = self.dll_list.current_item();
        if cur.is_null() {
            return None;
        }
        let version = cur.text(0).to_std_string().trim_start_matches('v').to_string();
        app.st.borrow().dll.as_ref()?.installed.iter().find(|d| d.version == version).cloned()
    }

    pub fn check_dll(&self, app: &Rc<App>) {
        unsafe {
            self.dll_latest.set_text(&qs("Checking…"));
        }
        let ctx = app.ctx.clone();
        app.bus.spawn(move |_| Some(UiEvent::DllRemote(dll::dll_check_update(&ctx))));
    }

    pub unsafe fn on_dll_remote(&self, _app: &Rc<App>, r: Result<RemoteDll, String>) {
        match r {
            Ok(remote) => {
                self.dll_latest.set_text(&qs(format!("Latest: v{}{}", remote.version, if remote.installed { " (installed)" } else { "" })));
                self.dll_latest.set_tool_tip(&qs(&remote.notes));
                self.dll_install.set_visible(!remote.installed);
                self.dll_install.set_enabled(true);
                *self.remote.borrow_mut() = Some(remote);
            }
            Err(e) => {
                self.dll_latest.set_text(&qs(e));
                self.dll_install.set_visible(false);
            }
        }
    }

    pub unsafe fn on_dll_progress(&self, f: f64) {
        self.dll_progress.set_value((f * 100.0) as i32);
    }

    pub unsafe fn on_dll_installed(&self, app: &Rc<App>, r: Result<InstalledDll, String>) {
        self.dll_progress.set_visible(false);
        self.dll_install.set_enabled(true);
        match r {
            Ok(d) => {
                self.dll_install.set_visible(false);
                self.dll_latest.set_text(&qs(format!(
                    "Installed v{}{}",
                    d.version,
                    if d.compatible { " (matches this game build)" } else { " (built for a different game build)" }
                )));
            }
            Err(e) => crate::dialogs::error(&app.window, &e),
        }
    }

    pub unsafe fn on_hash_progress(&self, p: &Progress) {
        self.profile_msg.set_text(&qs(format!("Hashing {} — {} of {}", p.file, format_bytes(p.done_bytes), format_bytes(p.total_bytes))));
    }

    pub unsafe fn on_hashed(&self, app: &Rc<App>, purpose: HashPurpose, r: Result<Vec<PackHash>, String>) {
        self.profile_msg.set_text(&qs(""));
        let hashes = match r {
            Ok(h) => h,
            Err(e) => return crate::dialogs::error(&app.window, &e),
        };
        match purpose {
            HashPurpose::Export => {
                let (name, date) = {
                    let st = app.st.borrow();
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                    let (y, m, d) = ymd(now);
                    (st.profiles.active.clone(), format!("{y}-{m:02}-{d:02}"))
                };
                let text = sync::build_export(&name, &hashes, &date);
                crate::dialogs::show_text(&app.window, "Export profile", "Send this to your co-op partner. They use \"Verify against an export\" to compare.", &text);
            }
            HashPurpose::Verify => {
                let text = app.verify_text.borrow().clone();
                let rows = sync::parse_export(&text);
                let (result, st_mods) = {
                    let st = app.st.borrow();
                    let enabled: Vec<String> = st.active().entries.iter().filter(|e| e.enabled && !e.is_separator() && st.module(&e.key).is_some()).map(|e| e.key.clone()).collect();
                    let map: HashMap<String, String> = hashes.iter().map(|h| (h.key.clone(), h.sha256.clone())).collect();
                    (sync::verify_export(&rows, &st.mods, &enabled, &map), st.mods.len())
                };
                let _ = st_mods;
                crate::dialogs::custom(&app.window, "Co-op sync check", 760, 520, |l, _| {
                    let head = label(if result.ok { "In sync: same packs, same files, same order." } else { "Not in sync:" });
                    bold(&head);
                    colored(&head, &if result.ok { icons::green() } else { icons::red() });
                    l.add_widget(&head);
                    let t = QTreeWidget::new_0a();
                    let h = QListOfQString::new_0a();
                    h.append_q_string(&qs("Status"));
                    h.append_q_string(&qs("Pack"));
                    t.set_header_labels(&h);
                    t.set_root_is_decorated(false);
                    t.header().set_section_resize_mode_2a(1, ResizeMode::Stretch);
                    for r in &result.rows {
                        let it = QTreeWidgetItem::new();
                        it.set_text(0, &qs(r.status.label()));
                        it.set_text(1, &qs(&r.row.file));
                        let c = match r.status {
                            SyncStatus::Match => icons::green(),
                            SyncStatus::Unverified => icons::grey(),
                            SyncStatus::Disabled => icons::amber(),
                            _ => icons::red(),
                        };
                        it.set_foreground(0, &qt_gui::QBrush::from_q_color(&c));
                        t.add_top_level_item(it.into_ptr());
                    }
                    for m in &result.extra {
                        let it = QTreeWidgetItem::new();
                        it.set_text(0, &qs("only on your side"));
                        it.set_text(1, &qs(&m.file));
                        it.set_foreground(0, &qt_gui::QBrush::from_q_color(&icons::amber()));
                        t.add_top_level_item(it.into_ptr());
                    }
                    l.add_widget_2a(&t, 1);
                    if result.order_differs {
                        l.add_widget(&label("The common packs load in a different order on your side."));
                    }
                });
            }
        }
    }

    unsafe fn import_ca(&self, app: &Rc<App>) {
        let mut file = app.st.borrow().paths.used_mods_file.clone().unwrap_or_default();
        if file.is_empty() {
            file = QFileDialog::get_open_file_name_4a(&app.window, &qs("Select used_mods.txt"), &qs(""), &qs("Mod list (*.txt)")).to_std_string();
            if file.is_empty() {
                return;
            }
        }
        let Some(name) = crate::dialogs::ask_text(&app.window, "Import CA launcher list", "Name for the new profile:", "CA launcher") else { return };
        match ops::import_mod_list(&app.ctx, &file) {
            Ok(imported) => {
                let keys: Vec<String> = imported.iter().filter_map(|i| i.key.clone()).collect();
                let missing: Vec<String> = imported.iter().filter(|i| i.key.is_none()).map(|i| i.file.clone()).collect();
                match self.make_profile(app, &name, &keys) {
                    Ok(()) => self.profile_msg.set_text(&qs(format!(
                        "Imported {} mods into \"{name}\"{}",
                        keys.len(),
                        if missing.is_empty() { String::new() } else { format!("; not installed: {}", missing.join(", ")) }
                    ))),
                    Err(e) => crate::dialogs::error(&app.window, &e),
                }
            }
            Err(e) => crate::dialogs::error(&app.window, &e),
        }
    }

    unsafe fn import_text(&self, app: &Rc<App>) {
        let Some(text) = crate::dialogs::ask_multiline(&app.window, "Import from text", "Paste a profile export (one pack per line).", "Next") else { return };
        let rows = sync::parse_export(&text);
        if rows.is_empty() {
            return crate::dialogs::error(&app.window, "Nothing to import.");
        }
        let Some(name) = crate::dialogs::ask_text(&app.window, "Import from text", "Name for the new profile:", "Imported") else { return };
        let keys: Vec<String> = {
            let st = app.st.borrow();
            rows.iter()
                .map(|r| {
                    if r.key.starts_with("ws:") {
                        r.key.clone()
                    } else {
                        sync::match_local(r, &st.mods).map(|m| m.key.clone()).unwrap_or_else(|| r.key.clone())
                    }
                })
                .collect()
        };
        let unknown = keys.iter().filter(|k| app.st.borrow().module(k).is_none()).count();
        match self.make_profile(app, &name, &keys) {
            Ok(()) => self.profile_msg.set_text(&qs(format!(
                "Imported {} mods into \"{name}\"{}",
                keys.len() - unknown,
                if unknown > 0 { format!("; {unknown} not installed (kept as missing)") } else { String::new() }
            ))),
            Err(e) => crate::dialogs::error(&app.window, &e),
        }
    }

    pub unsafe fn on_collection(&self, app: &Rc<App>, r: Result<CollectionResult, String>) {
        let res = match r {
            Ok(r) => r,
            Err(e) => {
                self.profile_msg.set_text(&qs(""));
                return crate::dialogs::error(&app.window, &e);
            }
        };
        app.st.borrow_mut().workshop.extend(res.items.clone());
        let (keys, missing): (Vec<String>, Vec<String>) = {
            let st = app.st.borrow();
            let keys = res.children.iter().filter_map(|id| st.mods.iter().find(|m| m.workshop_id.as_deref() == Some(id.as_str())).map(|m| m.key.clone())).collect();
            let missing = res
                .children
                .iter()
                .filter(|id| !st.mods.iter().any(|m| m.workshop_id.as_deref() == Some(id.as_str())))
                .map(|id| res.items.get(id).map(|i| i.title.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| id.clone()))
                .collect();
            (keys, missing)
        };
        let default = if res.title.is_empty() { format!("Collection {}", res.id) } else { res.title.clone() };
        let Some(name) = crate::dialogs::ask_text(&app.window, "Import Workshop collection", "Name for the new profile:", &default) else {
            self.profile_msg.set_text(&qs(""));
            return;
        };
        match self.make_profile(app, &name, &keys) {
            Ok(()) => {
                self.profile_msg.set_text(&qs(format!("\"{name}\": {} of {} collection items are installed and enabled.", keys.len(), res.children.len())));
                if !missing.is_empty() {
                    crate::dialogs::show_text(&app.window, "Not installed", "Subscribe to these on the Workshop, then use Refresh mods:", &missing.join("\n"));
                }
            }
            Err(e) => crate::dialogs::error(&app.window, &e),
        }
    }

    unsafe fn make_profile(&self, app: &Rc<App>, name: &str, keys: &[String]) -> Result<(), String> {
        {
            let mut st = app.st.borrow_mut();
            let mods = st.mods.clone();
            profile_ops::create(&mut st.profiles, name, None, &mods)?;
            st.update_active(|p| p.entries = profile_ops::enable_first(&p.entries, keys));
        }
        app.mark(DIRTY_ALL);
        Ok(())
    }

    /// Fill the page from the current settings / state.
    pub unsafe fn refresh(&self, app: &Rc<App>) {
        let s = app.ctx.settings();
        let st = app.st.borrow();
        self.game_root.set_text(&qs(format!(
            "{}{}",
            st.paths.game_root.clone().unwrap_or_else(|| "not found".into()),
            if s.game_root.is_some() { "  (manual)" } else { "  (auto)" }
        )));
        self.ws_dir.set_text(&qs(st.paths.workshop_dir.clone().unwrap_or_else(|| "not found".into())));
        self.version.set_text(&qs(st.paths.exe_version.clone().unwrap_or_else(|| "?".into())));
        self.game_auto.set_enabled(s.game_root.is_some());
        self.ws_auto.set_enabled(s.workshop_dir.is_some());
        self.folders.clear();
        for d in &s.extra_mod_dirs {
            self.folders.add_item_q_string(&qs(d));
        }
        let cutoff = st.cutoff(s.outdated_before);
        if cutoff > 0 {
            let (y, m, d) = ymd(cutoff);
            self.cutoff.set_date(&QDate::new_3a(y as i32, m as i32, d as i32));
        }
        self.cutoff_reset.set_enabled(s.outdated_before.is_some());
        self.tray.set_checked(s.minimize_to_tray);
        self.app_updates.set_checked(s.check_app_updates);
        self.dll_updates.set_checked(s.check_dll_updates);
        self.dll_auto.set_checked(s.auto_inject_external);
        self.cache_hours.set_value(s.workshop_cache_hours as i32);
        let ch = self.dll_channel.find_data_1a(&QVariant::from_q_string(&qs(&s.dll_channel)));
        self.dll_channel.set_current_index(ch.max(0));

        self.dll_list.clear();
        if let Some(d) = &st.dll {
            self.dll_game.set_text(&qs(format!(
                "Game build {} / {}. A DLL is only injected when its manifest matches these values, and never into a game that already has it loaded.",
                d.game_timestamp_hex.clone().unwrap_or_else(|| "?".into()),
                d.game_size_hex.clone().unwrap_or_else(|| "?".into())
            )));
            for i in &d.installed {
                let it = QTreeWidgetItem::new();
                it.set_text(0, &qs(format!("v{}", i.version)));
                it.set_text(1, &qs(i.manifest.as_ref().map(|m| if m.game_exe_version.is_empty() { m.exe_timestamp.clone() } else { m.game_exe_version.clone() }).unwrap_or_else(|| "no manifest".into())));
                let selected = d.selected.as_ref().map(|s| s.version == i.version).unwrap_or(false);
                it.set_text(2, &qs(if selected { "matches game · will inject" } else if i.compatible { "matches game" } else { "other game build" }));
                it.set_foreground(2, &qt_gui::QBrush::from_q_color(&if i.compatible { icons::green() } else { icons::red() }));
                self.dll_list.add_top_level_item(it.into_ptr());
            }
        }
        let cfg = dll::dll_read_cfg();
        self.cfg_build.set_text(&qs(&cfg.build_number));
        self.cfg_short.set_text(&qs(&cfg.build_number_short));
        let mi = self.cfg_modified.find_data_1a(&QVariant::from_q_string(&qs(match cfg.build_modified {
            Some(false) => "0",
            Some(true) => "1",
            None => "",
        })));
        self.cfg_modified.set_current_index(mi.max(0));

        let profile = st.active();
        drop(st);
        self.preview.set_plain_text(&qs(ops::preview_mod_list(&app.ctx, &profile)));
        self.about.set_text(&qs(format!(
            "TK Mod Manager v{}. Data in {}. Shortcuts: --profile \"Name\" --launch starts a profile directly; --minimized starts in the tray.",
            ops::APP_VERSION,
            paths::app_data_dir().display()
        )));
        let _ = DIRTY_LIST;
    }
}

#[allow(dead_code)]
fn unused(_: QHBoxLayout) {}
