//! The main window and the event pump.
//!
//! Rules: slots only change `State` and mark parts of the UI dirty; the 50 ms pump drains worker
//! events and redraws the dirty parts. Widgets are therefore never rebuilt from inside one of
//! their own signals.

use crate::dialogs::UpdateChoice;
use crate::events::{Bus, HashPurpose, UiEvent};
use crate::state::State;
use crate::{details, launch_panel, mod_list, settings_ui, tabs, tray};
use cpp_core::{Ptr, StaticUpcast};
use qt_core::{qs, QBox, QObject, QTimer, SlotNoArgs, SlotOfInt};
use qt_gui::{QGuiApplication, QIcon, QPixmap};
use qt_widgets::{
    q_message_box::StandardButton, QComboBox, QHBoxLayout, QLabel, QMainWindow, QMenu, QMessageBox, QPushButton, QScrollArea, QSplitter,
    QStackedWidget, QTabBar, QToolButton, QVBoxLayout, QWidget,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tkmm_core::cli::StartupArgs;
use tkmm_core::launch::{self, Emit};
use tkmm_core::{dll, ops, profile_ops, se_scan, update, workshop, Ctx};

pub const DIRTY_LIST: u32 = 1;
pub const DIRTY_DETAILS: u32 = 2;
pub const DIRTY_LAUNCH: u32 = 4;
pub const DIRTY_HEADER: u32 = 8;
pub const DIRTY_SETTINGS: u32 = 16;
pub const DIRTY_ALL: u32 = 31;

pub const PAGE_CONFLICTS: i32 = 1;
pub const PAGE_LOGS: i32 = 2;
pub const PAGE_SETTINGS: i32 = 3;

pub struct App {
    pub ctx: Ctx,
    pub bus: Bus,
    rx: RefCell<Receiver<UiEvent>>,
    pub st: RefCell<State>,
    pub dirty: Cell<u32>,
    /// Set while the UI is being filled programmatically: signal handlers ignore changes.
    pub suppress: Cell<bool>,
    pub startup: RefCell<Option<StartupArgs>>,
    /// Window may stay hidden without quitting (started with --minimized).
    pub hidden_ok: Cell<bool>,
    pub last_poll: Cell<Instant>,
    pub last_logs: Cell<Instant>,
    pub notes_pending: RefCell<Option<(String, String, Instant)>>,
    pub verify_text: RefCell<String>,
    pub update_meta: RefCell<Option<update::UpdateMeta>>,

    pub window: QBox<QMainWindow>,
    pub tabs: QBox<QTabBar>,
    pub profile_combo: QBox<QComboBox>,
    pub profile_btn: QBox<QToolButton>,
    pub profile_menu: QBox<QMenu>,
    pub settings_btn: QBox<QPushButton>,
    pub banner: QBox<QLabel>,
    pub update_bar: QBox<QWidget>,
    pub update_label: QBox<QLabel>,
    pub update_btn: QBox<QPushButton>,
    pub stack: QBox<QStackedWidget>,
    pub details_area: QBox<QScrollArea>,
    pub launch_layout: QBox<QVBoxLayout>,
    pub ml: mod_list::ModListUi,
    pub conflicts: tabs::ConflictsUi,
    pub logs: tabs::LogsUi,
    pub settings: settings_ui::SettingsUi,
    pub tray: tray::TrayUi,
}

impl StaticUpcast<QObject> for App {
    unsafe fn static_upcast(ptr: Ptr<Self>) -> Ptr<QObject> {
        ptr.window.as_ptr().static_upcast()
    }
}

fn app_update_text(meta: &update::UpdateMeta) -> String {
    let text = format!(
        "Download and install TK Mod Manager v{}?\n\nThe app will close and restart when it is done.\n\n{}",
        meta.version,
        crate::dialogs::clip_notes(&meta.notes)
    );
    text.trim_end().to_string()
}

pub fn app_icon() -> cpp_core::CppBox<QIcon> {
    unsafe {
        let pm = QPixmap::new();
        let bytes = include_bytes!("../icons/128x128.png");
        pm.load_from_data_uchar_uint(bytes.as_ptr(), bytes.len() as u32);
        QIcon::from_q_pixmap(&pm)
    }
}

impl App {
    pub fn new(ctx: Ctx, bus: Bus, rx: Receiver<UiEvent>, startup: StartupArgs) -> Rc<App> {
        unsafe {
            let window = QMainWindow::new_0a();
            window.set_window_title(&qs("TK Mod Manager"));
            window.set_window_icon(&app_icon());
            window.resize_2a(1400, 900);
            window.set_minimum_size_2a(1000, 620);

            // Left column: header over the stacked pages.
            let left = QWidget::new_0a();
            let left_layout = QVBoxLayout::new_1a(&left);
            left_layout.set_contents_margins_4a(8, 8, 4, 8);
            left_layout.set_spacing(6);

            let header = QHBoxLayout::new_0a();
            let title = QLabel::from_q_string(&qs("TK Mod Manager"));
            let f = qt_gui::QFont::new_copy(title.font());
            f.set_bold(true);
            f.set_point_size(f.point_size() + 2);
            title.set_font(&f);
            header.add_widget(&title);
            header.add_spacing(12);
            let tabs = QTabBar::new_0a();
            tabs.add_tab_1a(&qs("Mods"));
            tabs.add_tab_1a(&qs("Conflicts"));
            tabs.add_tab_1a(&qs("Logs"));
            tabs.set_draw_base(false);
            header.add_widget(&tabs);
            header.add_stretch_1a(1);
            header.add_widget(QLabel::from_q_string(&qs("Profile")).into_ptr());
            let profile_combo = QComboBox::new_0a();
            profile_combo.set_minimum_width(220);
            header.add_widget(&profile_combo);
            let profile_btn = QToolButton::new_0a();
            profile_btn.set_text(&qs("Actions"));
            profile_btn.set_tool_tip(&qs("Profile actions"));
            profile_btn.set_popup_mode(qt_widgets::q_tool_button::ToolButtonPopupMode::InstantPopup);
            let profile_menu = QMenu::new();
            profile_btn.set_menu(&profile_menu);
            header.add_widget(&profile_btn);
            header.add_stretch_1a(1);
            let settings_btn = QPushButton::from_q_string(&qs("Settings"));
            settings_btn.set_checkable(true);
            header.add_widget(&settings_btn);
            left_layout.add_layout_1a(&header);

            let banner = QLabel::new();
            banner.set_word_wrap(true);
            banner.set_visible(false);
            left_layout.add_widget(&banner);

            let update_bar = QWidget::new_0a();
            let ub = QHBoxLayout::new_1a(&update_bar);
            ub.set_contents_margins_4a(0, 0, 0, 0);
            let update_label = QLabel::new();
            let update_btn = QPushButton::from_q_string(&qs("Install and restart"));
            ub.add_widget(&update_label);
            ub.add_stretch_1a(1);
            ub.add_widget(&update_btn);
            update_bar.set_visible(false);
            left_layout.add_widget(&update_bar);

            let stack = QStackedWidget::new_0a();
            let ml = mod_list::ModListUi::build();
            let conflicts = tabs::ConflictsUi::build();
            let logs = tabs::LogsUi::build();
            let settings = settings_ui::SettingsUi::build();
            stack.add_widget(&ml.root);
            stack.add_widget(&conflicts.root);
            stack.add_widget(&logs.root);
            stack.add_widget(&settings.root);
            left_layout.add_widget_2a(&stack, 1);

            // Right column: details (scrolls) above the launch controls.
            let right = QWidget::new_0a();
            let right_layout = QVBoxLayout::new_1a(&right);
            right_layout.set_contents_margins_4a(4, 8, 8, 8);
            right_layout.set_spacing(6);
            let details_area = QScrollArea::new_0a();
            details_area.set_widget_resizable(true);
            details_area.set_frame_shape(qt_widgets::q_frame::Shape::NoFrame);
            right_layout.add_widget_2a(&details_area, 1);
            let launch_area = QWidget::new_0a();
            let launch_layout = QVBoxLayout::new_1a(&launch_area);
            launch_layout.set_contents_margins_4a(0, 0, 0, 0);
            right_layout.add_widget(&launch_area);
            right.set_minimum_width(300);
            right.set_maximum_width(420);

            let split = QSplitter::new();
            split.add_widget(&left);
            split.add_widget(&right);
            split.set_stretch_factor(0, 1);
            split.set_stretch_factor(1, 0);
            split.set_children_collapsible(false);
            window.set_central_widget(&split);

            let tray = tray::TrayUi::build(&app_icon());
            let st = State::load();
            let hidden_ok = startup.minimized;

            Rc::new(App {
                ctx,
                bus,
                rx: RefCell::new(rx),
                st: RefCell::new(st),
                dirty: Cell::new(DIRTY_ALL),
                suppress: Cell::new(false),
                startup: RefCell::new(Some(startup)),
                hidden_ok: Cell::new(hidden_ok),
                last_poll: Cell::new(Instant::now()),
                last_logs: Cell::new(Instant::now() - Duration::from_secs(10)),
                notes_pending: RefCell::new(None),
                verify_text: RefCell::new(String::new()),
                update_meta: RefCell::new(None),
                window,
                tabs,
                profile_combo,
                profile_btn,
                profile_menu,
                settings_btn,
                banner,
                update_bar,
                update_label,
                update_btn,
                stack,
                details_area,
                launch_layout,
                ml,
                conflicts,
                logs,
                settings,
                tray,
            })
        }
    }

    pub fn mark(&self, bits: u32) {
        self.dirty.set(self.dirty.get() | bits);
    }

    pub fn page(&self) -> i32 {
        unsafe { self.stack.current_index() }
    }

    pub unsafe fn set_page(self: &Rc<Self>, page: i32) {
        self.suppress.set(true);
        self.stack.set_current_index(page);
        self.settings_btn.set_checked(page == PAGE_SETTINGS);
        if page != PAGE_SETTINGS {
            self.tabs.set_current_index(page);
        }
        self.suppress.set(false);
        if page == PAGE_CONFLICTS {
            self.run_conflicts();
        }
        if page == PAGE_LOGS {
            self.last_logs.set(Instant::now() - Duration::from_secs(10));
        }
        if page == PAGE_SETTINGS {
            self.mark(DIRTY_SETTINGS);
        }
    }

    /// Wire every signal, start the pump and the background jobs, show the window.
    pub unsafe fn start(self: &Rc<Self>) {
        let this = self.clone();
        self.tabs.current_changed().connect(&SlotOfInt::new(&self.window, move |i| {
            if !this.suppress.get() {
                this.set_page(i);
            }
        }));
        let this = self.clone();
        self.settings_btn.clicked().connect(&SlotNoArgs::new(&self.window, move || {
            let target = if this.page() == PAGE_SETTINGS { this.tabs.current_index() } else { PAGE_SETTINGS };
            this.set_page(target);
        }));
        let this = self.clone();
        self.profile_combo.activated().connect(&SlotOfInt::new(&self.window, move |i| {
            if this.suppress.get() || i < 0 {
                return;
            }
            let name = this.profile_combo.item_data_1a(i).to_string().to_std_string();
            this.switch_profile(&name);
        }));
        self.build_profile_menu();
        let this = self.clone();
        self.update_btn.clicked().connect(&SlotNoArgs::new(&self.window, move || this.confirm_app_update()));

        self.ml.connect(self);
        self.conflicts.connect(self);
        self.logs.connect(self);
        self.settings.connect(self);
        self.tray.connect(self);

        let timer = QTimer::new_1a(&self.window);
        let this = self.clone();
        timer.timeout().connect(&SlotNoArgs::new(&self.window, move || this.pump()));
        timer.start_1a(50);
        std::mem::forget(timer); // parented to the window

        self.rescan();
        self.refresh_dll();
        let s = self.ctx.settings();
        // Startup checks offer each update in a popup. The DLL check waits for the app
        // check's answer (see UiEvent::AppUpdate) so the two popups never stack.
        if s.check_app_updates {
            self.check_app_update(false);
        } else if s.check_dll_updates {
            self.settings.check_dll(self, true);
        }
        let bus = self.bus.clone();
        let emit: Emit = Arc::new(move |st| bus.send(UiEvent::Launch(st)));
        launch::start_external_watcher(self.ctx.clone(), emit);
        self.st.borrow_mut().game_running = launch::game_running();

        self.tray.set_visible(s.minimize_to_tray || self.hidden_ok.get());
        if !self.hidden_ok.get() {
            self.window.show();
        }
    }

    // ------------------------------------------------------------------ background jobs

    pub fn rescan(self: &Rc<Self>) {
        let ctx = self.ctx.clone();
        self.bus.spawn(move |_| {
            let paths = ctx.game_paths();
            let mods = ctx.scan();
            Some(UiEvent::Scanned { paths, mods, installs: tkmm_core::paths::detect_installs() })
        });
    }

    pub fn refresh_dll(self: &Rc<Self>) {
        let ctx = self.ctx.clone();
        self.bus.spawn(move |_| Some(UiEvent::Dll(dll::dll_status(&ctx))));
    }

    pub fn refresh_se_scan(self: &Rc<Self>) {
        let ctx = self.ctx.clone();
        self.bus.spawn(move |_| Some(UiEvent::SeScan(se_scan::se_requirements(&ctx, &[]))));
    }

    fn fetch_workshop(self: &Rc<Self>) {
        let ids: Vec<String> = {
            let st = self.st.borrow();
            let mut v: Vec<String> = st.mods.iter().filter_map(|m| m.workshop_id.clone()).collect();
            v.sort();
            v.dedup();
            v
        };
        let ctx = self.ctx.clone();
        self.bus.spawn(move |bus| {
            bus.send(UiEvent::Workshop(workshop::workshop_cached()));
            workshop::workshop_fetch(&ctx, &ids, false).ok().map(UiEvent::Workshop)
        });
    }

    pub fn refresh_saves(self: &Rc<Self>) {
        let store = self.st.borrow().paths.store;
        self.bus.spawn(move |_| Some(UiEvent::Saves(launch::list_saves(store))));
    }

    pub fn check_app_update(self: &Rc<Self>, verbose: bool) {
        let pre = self.ctx.settings().app_channel == "prerelease";
        self.bus.spawn(move |_| Some(UiEvent::AppUpdate(update::check_update(pre), verbose)));
    }

    /// The update bar's button: asks, then installs.
    pub unsafe fn confirm_app_update(self: &Rc<Self>) {
        let Some(meta) = self.update_meta.borrow().clone() else { return };
        if crate::dialogs::confirm(&self.window, "Update TK Mod Manager", &app_update_text(&meta)) {
            self.install_app_update();
        }
    }

    /// Startup offer: Install / Skip this version / Not now. Returns whether it is installing.
    unsafe fn offer_app_update(self: &Rc<Self>, meta: &update::UpdateMeta) -> bool {
        if self.ctx.settings().skipped_app_version == meta.version {
            return false;
        }
        match crate::dialogs::ask_update(&self.window, "Update TK Mod Manager", &app_update_text(meta)) {
            UpdateChoice::Install => {
                self.install_app_update();
                true
            }
            UpdateChoice::Skip => {
                let mut s = self.ctx.settings();
                s.skipped_app_version = meta.version.clone();
                if let Err(e) = self.ctx.set_settings(s) {
                    crate::dialogs::error(&self.window, &e);
                }
                false
            }
            UpdateChoice::Later => false,
        }
    }

    /// Download and install (no questions; callers have asked).
    unsafe fn install_app_update(self: &Rc<Self>) {
        let Some(meta) = self.update_meta.borrow().clone() else { return };
        self.update_btn.set_enabled(false);
        self.update_label.set_text(&qs(format!("Downloading v{}…", meta.version)));
        self.bus.spawn(move |bus| {
            let b = bus.clone();
            let r = update::install_update(&meta.asset_url, &move |f| b.send(UiEvent::UpdateProgress(f)));
            Some(UiEvent::UpdateInstalled(r))
        });
    }

    pub fn run_conflicts(self: &Rc<Self>) {
        let keys: Vec<String> = {
            let st = self.st.borrow();
            st.active().entries.iter().filter(|e| e.enabled && !e.is_separator() && st.module(&e.key).is_some()).map(|e| e.key.clone()).collect()
        };
        unsafe { self.conflicts.set_busy(true) };
        let ctx = self.ctx.clone();
        self.bus.spawn(move |_| Some(UiEvent::Conflicts(tkmm_core::conflicts::conflicts_for(&ctx, &keys))));
    }

    pub fn hash_enabled(self: &Rc<Self>, purpose: HashPurpose) {
        let keys: Vec<String> = {
            let st = self.st.borrow();
            st.active().entries.iter().filter(|e| e.enabled && !e.is_separator() && st.module(&e.key).is_some()).map(|e| e.key.clone()).collect()
        };
        let ctx = self.ctx.clone();
        self.bus.spawn(move |bus| {
            let b = bus.clone();
            let r = tkmm_core::hash::hash_packs(&ctx, &keys, &move |p| b.send(UiEvent::HashProgress(p)));
            Some(UiEvent::Hashed(purpose, r))
        });
    }

    // ------------------------------------------------------------------ actions

    pub unsafe fn switch_profile(self: &Rc<Self>, name: &str) {
        {
            let mut st = self.st.borrow_mut();
            if st.profiles.profiles.iter().any(|p| p.name == name) {
                st.profiles.active = name.to_string();
                st.selected.clear();
                st.focused = None;
                st.save_profiles();
            }
        }
        self.mark(DIRTY_ALL);
    }

    pub unsafe fn launch(self: &Rc<Self>, profile_name: Option<String>) {
        if let Some(name) = profile_name.filter(|n| !n.is_empty()) {
            let exists = self.st.borrow().profiles.profiles.iter().any(|p| p.name == name);
            if !exists {
                self.set_launch_message("failed", &format!("No profile named \"{name}\""));
                return;
            }
            self.switch_profile(&name);
        }
        let (profile, save) = {
            let st = self.st.borrow();
            (st.active(), Some(st.load_save.clone()).filter(|s| !s.is_empty()))
        };
        // Epic goes through CA's launcher, whose own ticked mods load on top of our list.
        if self.st.borrow().paths.store == tkmm_core::paths::GameStore::Epic {
            let ticked = launch::ca_selected_mods(tkmm_core::paths::GameStore::Epic);
            if !ticked.is_empty()
                && !crate::dialogs::confirm(
                    &self.window,
                    "CA's launcher has mods ticked",
                    &format!(
                        "CA's launcher will also load {} mod{} of its own:\n\n{}\n\nThey load on top of this profile and can upset the load order. Untick them in CA's launcher for a clean run.\n\nStart anyway?",
                        ticked.len(),
                        if ticked.len() == 1 { "" } else { "s" },
                        ticked.join("\n")
                    ),
                )
            {
                return;
            }
        }
        self.set_launch_message("writing", "Starting…");
        let ctx = self.ctx.clone();
        let bus = self.bus.clone();
        let emit: Emit = Arc::new(move |s| bus.send(UiEvent::Launch(s)));
        match launch::launch_game(&ctx, &emit, &profile, save.as_deref()) {
            Ok(_) => {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let mut st = self.st.borrow_mut();
                st.game_running = true;
                st.crash = None;
                st.update_active(|p| p.last_played = Some(now));
            }
            Err(e) => self.set_launch_message("failed", &e),
        }
        self.mark(DIRTY_LAUNCH | DIRTY_LIST);
    }

    /// Switch to another copy of the game (Steam / Epic / Game Pass) and rescan.
    pub unsafe fn set_game_root(self: &Rc<Self>, root: &str) {
        if self.st.borrow().game_running {
            crate::dialogs::error(&self.window, "Quit the game before switching to another copy.");
            self.mark(DIRTY_ALL);
            return;
        }
        let mut s = self.ctx.settings();
        s.game_root = Some(root.to_string());
        if let Err(e) = self.ctx.set_settings(s) {
            crate::dialogs::error(&self.window, &e);
            return;
        }
        self.rescan();
        self.refresh_dll();
        self.refresh_saves();
        self.mark(DIRTY_ALL);
    }

    pub fn set_launch_message(&self, phase: &str, msg: &str) {
        self.st.borrow_mut().launch = Some(tkmm_core::launch::LaunchStatus { phase: phase.into(), message: msg.into(), pid: None, exit_code: None, history_id: None });
        self.mark(DIRTY_LAUNCH);
    }

    unsafe fn build_profile_menu(self: &Rc<Self>) {
        let m = &self.profile_menu;
        let add = |label: &str, f: Box<dyn Fn()>| {
            let a = m.add_action_q_string(&qs(label));
            a.triggered().connect(&SlotNoArgs::new(&self.window, move || f()));
        };
        let this = self.clone();
        add("New profile…", Box::new(move || this.profile_new(false)));
        let this = self.clone();
        add("Duplicate…", Box::new(move || this.profile_new(true)));
        let this = self.clone();
        add("Rename…", Box::new(move || this.profile_rename()));
        let this = self.clone();
        add("Delete", Box::new(move || this.profile_delete()));
        m.add_separator();
        let this = self.clone();
        add("Create desktop shortcut", Box::new(move || {
            let name = this.st.borrow().profiles.active.clone();
            match tkmm_core::cli::create_profile_shortcut(&name) {
                Ok(p) => crate::dialogs::info(&this.window, "Shortcut created", &format!("Shortcut created:\n{p}")),
                Err(e) => crate::dialogs::error(&this.window, &e),
            }
        }));
    }

    unsafe fn profile_new(self: &Rc<Self>, duplicate: bool) {
        let active = self.st.borrow().profiles.active.clone();
        let initial = if duplicate { format!("{active} copy") } else { String::new() };
        let Some(name) = crate::dialogs::ask_text(&self.window, if duplicate { "Duplicate profile" } else { "New profile" }, "Profile name:", &initial) else { return };
        let r = {
            let mut st = self.st.borrow_mut();
            let from = duplicate.then(|| st.active());
            let mods = st.mods.clone();
            let r = profile_ops::create(&mut st.profiles, &name, from.as_ref(), &mods);
            if r.is_ok() {
                st.save_profiles();
            }
            r
        };
        match r {
            Ok(()) => self.mark(DIRTY_ALL),
            Err(e) => crate::dialogs::error(&self.window, &e),
        }
    }

    unsafe fn profile_rename(self: &Rc<Self>) {
        let active = self.st.borrow().profiles.active.clone();
        let Some(name) = crate::dialogs::ask_text(&self.window, "Rename profile", "New name:", &active) else { return };
        let r = {
            let mut st = self.st.borrow_mut();
            let r = profile_ops::rename(&mut st.profiles, &active, &name);
            if r.is_ok() {
                st.save_profiles();
            }
            r
        };
        match r {
            Ok(()) => self.mark(DIRTY_ALL),
            Err(e) => crate::dialogs::error(&self.window, &e),
        }
    }

    unsafe fn profile_delete(self: &Rc<Self>) {
        let active = self.st.borrow().profiles.active.clone();
        if !crate::dialogs::confirm(&self.window, "Delete profile", &format!("Delete profile \"{active}\"?")) {
            return;
        }
        let r = {
            let mut st = self.st.borrow_mut();
            let r = profile_ops::delete(&mut st.profiles, &active);
            if r.is_ok() {
                st.save_profiles();
            }
            r
        };
        match r {
            Ok(()) => self.mark(DIRTY_ALL),
            Err(e) => crate::dialogs::error(&self.window, &e),
        }
    }

    /// Apply a forwarded or initial command line.
    pub unsafe fn apply_args(self: &Rc<Self>, args: StartupArgs) {
        if !args.minimized {
            self.show_window();
        }
        if args.launch {
            self.launch(args.profile);
        } else if let Some(p) = args.profile {
            self.switch_profile(&p);
        }
    }

    pub unsafe fn show_window(&self) {
        self.hidden_ok.set(false);
        self.window.show();
        self.window.raise();
        self.window.activate_window();
    }

    // ------------------------------------------------------------------ pump

    unsafe fn pump(self: &Rc<Self>) {
        loop {
            let ev = self.rx.borrow().try_recv();
            match ev {
                Ok(e) => self.handle(e),
                Err(_) => break,
            }
        }

        // Debounced notes save.
        let due = matches!(&*self.notes_pending.borrow(), Some((_, _, t)) if t.elapsed() > Duration::from_millis(800));
        if due {
            if let Some((key, text, _)) = self.notes_pending.borrow_mut().take() {
                self.st.borrow_mut().set_meta(&key, |m| m.notes = text);
            }
        }

        // Game-running safety poll (the launch thread reports games it follows).
        if self.last_poll.get().elapsed() > Duration::from_secs(5) {
            self.last_poll.set(Instant::now());
            let running = launch::game_running();
            let changed = {
                let mut st = self.st.borrow_mut();
                let changed = st.game_running != running;
                st.game_running = running;
                changed
            };
            if changed {
                if !running {
                    self.refresh_saves();
                }
                self.mark(DIRTY_LAUNCH | DIRTY_LIST);
            }
        }

        if self.page() == PAGE_LOGS && self.last_logs.get().elapsed() > Duration::from_secs(2) {
            self.last_logs.set(Instant::now());
            self.logs.poll(self);
        }

        // Closing the window quits unless the app lives in the tray.
        if !self.window.is_visible() && !self.hidden_ok.get() && !self.ctx.settings().minimize_to_tray {
            qt_core::QCoreApplication::quit();
            return;
        }

        self.ml.flush_drop(self);

        let mut bits = self.dirty.replace(0);
        if bits == 0 {
            return;
        }
        // Don't rebuild the list under a press or drag; try again on a later tick.
        if bits & DIRTY_LIST != 0 && qt_gui::QGuiApplication::mouse_buttons().to_int() != 0 {
            self.dirty.set(DIRTY_LIST);
            bits &= !DIRTY_LIST;
        }
        self.suppress.set(true);
        if bits & DIRTY_HEADER != 0 {
            self.refresh_header();
        }
        if bits & DIRTY_LIST != 0 {
            self.ml.refresh(self);
        }
        if bits & DIRTY_DETAILS != 0 {
            details::refresh(self);
        }
        if bits & DIRTY_LAUNCH != 0 {
            launch_panel::refresh(self);
        }
        if bits & DIRTY_SETTINGS != 0 && self.page() == PAGE_SETTINGS {
            self.settings.refresh(self);
        }
        self.suppress.set(false);
    }

    unsafe fn handle(self: &Rc<Self>, e: UiEvent) {
        match e {
            UiEvent::Scanned { paths, mods, installs } => {
                let first = self.st.borrow().mods.is_empty();
                {
                    let mut st = self.st.borrow_mut();
                    // Switching to another copy brings in that copy's own profiles. This also
                    // fires on the first scan, when the loaded file is still the default one.
                    if st.profiles_store != paths.store {
                        st.load_profiles_for(paths.store);
                    }
                    st.paths = paths;
                    st.installs = installs;
                    st.set_mods(mods);
                }
                self.mark(DIRTY_ALL);
                self.refresh_se_scan();
                if first {
                    // A leftover from a launch the app did not see out (closed mid-launch):
                    // drop it so a start straight from Epic does not reuse that profile.
                    let store = self.st.borrow().paths.store;
                    if !self.st.borrow().game_running {
                        launch::clear_user_script(store);
                    }
                    self.fetch_workshop();
                    self.refresh_saves();
                    self.seed_from_ca_launcher();
                    if let Some(args) = self.startup.borrow_mut().take() {
                        if args.launch || args.profile.is_some() {
                            self.apply_args(args);
                        }
                    }
                }
            }
            UiEvent::Workshop(items) => {
                self.st.borrow_mut().workshop.extend(items);
                self.mark(DIRTY_LIST | DIRTY_DETAILS | DIRTY_LAUNCH);
            }
            UiEvent::SeScan(map) => {
                self.st.borrow_mut().se_scan = map;
                self.mark(DIRTY_LIST | DIRTY_DETAILS | DIRTY_LAUNCH);
            }
            UiEvent::Dll(s) => {
                self.st.borrow_mut().dll = Some(s);
                self.mark(DIRTY_ALL);
            }
            UiEvent::DllRemote(r, startup) => self.settings.on_dll_remote(self, r, startup),
            UiEvent::DllCatalog(r) => self.settings.on_dll_catalog(self, r),
            UiEvent::DllProgress(f) => self.settings.on_dll_progress(f),
            UiEvent::DllInstalled(r) => {
                self.settings.on_dll_installed(self, r);
                self.refresh_dll();
            }
            UiEvent::Launch(s) => {
                let ended = matches!(s.phase.as_str(), "exited" | "crashed");
                {
                    let mut st = self.st.borrow_mut();
                    if s.phase == "crashed" {
                        st.crash = Some((s.history_id, s.exit_code));
                    }
                    if ended {
                        st.game_running = false;
                    } else if s.phase != "failed" {
                        st.game_running = true;
                    }
                    st.launch = Some(s);
                }
                if ended {
                    self.refresh_saves();
                }
                self.mark(DIRTY_LAUNCH | DIRTY_LIST);
            }
            UiEvent::Saves(s) => {
                self.st.borrow_mut().saves = s;
                self.mark(DIRTY_LAUNCH);
            }
            UiEvent::HashProgress(p) => self.settings.on_hash_progress(&p),
            UiEvent::Hashed(purpose, r) => self.settings.on_hashed(self, purpose, r),
            UiEvent::Conflicts(r) => self.conflicts.show_report(self, r),
            UiEvent::Collection(r) => self.settings.on_collection(self, r),
            UiEvent::AppUpdate(r, verbose) => {
                // Non-verbose = the startup check: offer the update in a popup, then move on
                // to the DLL check unless the app is about to update and restart.
                let mut updating = false;
                match r {
                    Ok(Some(meta)) => {
                        self.update_label.set_text(&qs(format!("TK Mod Manager v{} is available.", meta.version)));
                        self.update_label.set_tool_tip(&qs(&meta.notes));
                        self.update_btn.set_enabled(true);
                        self.update_bar.set_visible(true);
                        *self.update_meta.borrow_mut() = Some(meta);
                        if !verbose {
                            let meta = self.update_meta.borrow().clone();
                            updating = meta.map(|m| self.offer_app_update(&m)).unwrap_or(false);
                        }
                    }
                    Ok(None) if verbose => crate::dialogs::info(&self.window, "Updates", "TK Mod Manager is up to date."),
                    Err(e) if verbose => crate::dialogs::error(&self.window, &format!("Update check failed: {e}")),
                    _ => {}
                }
                if !verbose && !updating && self.ctx.settings().check_dll_updates {
                    self.settings.check_dll(self, true);
                }
            }
            UiEvent::UpdateProgress(f) => self.update_label.set_text(&qs(format!("Downloading update… {}%", (f * 100.0).round()))),
            UiEvent::UpdateInstalled(r) => match r {
                Ok(()) => {
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).spawn();
                    }
                    qt_core::QCoreApplication::quit();
                }
                Err(e) => {
                    self.update_btn.set_enabled(true);
                    self.update_label.set_text(&qs("Update failed"));
                    crate::dialogs::error(&self.window, &format!("The update could not be installed: {e}"));
                }
            },
            UiEvent::Cli(args) => self.apply_args(args),
        }
    }

    /// First run: enable what the CA launcher had enabled.
    unsafe fn seed_from_ca_launcher(self: &Rc<Self>) {
        let marker = tkmm_core::paths::app_data_dir().join(".seeded");
        if marker.exists() {
            return;
        }
        let file = self.st.borrow().paths.used_mods_file.clone();
        let untouched = self.st.borrow().active().entries.iter().all(|e| !e.enabled);
        if let (Some(file), true) = (file, untouched) {
            if let Ok(imported) = ops::import_mod_list(&self.ctx, &file) {
                let keys: Vec<String> = imported.into_iter().filter_map(|i| i.key).collect();
                if !keys.is_empty() {
                    self.st.borrow_mut().update_active(|p| p.entries = profile_ops::enable_first(&p.entries, &keys));
                    self.mark(DIRTY_ALL);
                }
            }
        }
        let _ = std::fs::write(marker, "1");
    }

    unsafe fn refresh_header(self: &Rc<Self>) {
        let st = self.st.borrow();
        self.profile_combo.clear();
        let mut current = 0;
        for (i, p) in st.profiles.profiles.iter().enumerate() {
            let n = p.entries.iter().filter(|e| e.enabled && !e.is_separator()).count();
            self.profile_combo.add_item_q_string_q_variant(&qs(format!("{} ({n})", p.name)), &qt_core::QVariant::from_q_string(&qs(&p.name)));
            if p.name == st.profiles.active {
                current = i as i32;
            }
        }
        self.profile_combo.set_current_index(current);
        self.profile_combo.set_enabled(!st.game_running);
        self.profile_btn.set_enabled(!st.game_running);
        let msg = if let Some(e) = &st.error {
            Some(e.clone())
        } else if st.paths.game_root.is_none() && !st.mods.is_empty() {
            Some("Three Kingdoms was not found in Steam, Epic or Game Pass. Set the game folder in Settings.".to_string())
        } else {
            None
        };
        self.banner.set_visible(msg.is_some());
        self.banner.set_text(&qs(msg.unwrap_or_default()));
        drop(st);
        self.tray.rebuild(self);
    }

    pub unsafe fn clipboard_set(&self, text: &str) {
        QGuiApplication::clipboard().set_text_1a(&qs(text));
    }

    #[allow(dead_code)]
    pub unsafe fn ask_yes_no(&self, title: &str, text: &str) -> bool {
        QMessageBox::question_q_widget2_q_string(&self.window, &qs(title), &qs(text)) == StandardButton::Yes
    }
}
