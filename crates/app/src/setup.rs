//! First-time setup: a few pages shown on the manager's first start on a PC (after the language
//! picker), each doing one thing with the buttons the rest of the app already has. Ends with
//! the choice of a guided tour. Existing users are offered the tour from a bar instead.

use crate::app::{App, DIRTY_ALL, DIRTY_LAUNCH};
use crate::details::{bold, colored, label};
use crate::icons;
use qt_core::{qs, QBox, SlotNoArgs};
use qt_widgets::{QCheckBox, QDialog, QHBoxLayout, QLabel, QPushButton, QStackedWidget, QVBoxLayout, QWidget};
use std::cell::Cell;
use std::rc::Rc;
use tkmm_core::settings::SETUP_VERSION;

/// One page: a title and a short text, then the page's own controls.
unsafe fn page(stack: &QBox<QStackedWidget>, title: &str, text: &str) -> QBox<QVBoxLayout> {
    let w = QWidget::new_0a();
    let v = QVBoxLayout::new_1a(&w);
    v.set_spacing(10);
    let t = label(title);
    bold(&t);
    let f = qt_gui::QFont::new_copy(t.font());
    f.set_point_size(f.point_size() + 3);
    t.set_font(&f);
    v.add_widget(&t);
    v.add_widget(&label(text));
    stack.add_widget(&w);
    v
}

unsafe fn button_row(v: &QBox<QVBoxLayout>, buttons: &[&QBox<QPushButton>]) {
    let r = QHBoxLayout::new_0a();
    v.add_layout_1a(&r);
    for b in buttons {
        r.add_widget(*b);
    }
    r.add_stretch_1a(1);
}

/// What the game page says about the copy the manager found.
unsafe fn game_text(app: &Rc<App>) -> (String, cpp_core::CppBox<qt_gui::QColor>) {
    let st = app.st.borrow();
    match &st.paths.game_root {
        Some(root) => (
            t!("setup-game-found", store = st.paths.store.label(), version = st.paths.exe_version.clone().unwrap_or_else(|| "?".into()), path = root),
            icons::green(),
        ),
        None => (t!("setup-game-missing"), icons::red()),
    }
}

/// What the script extender page says about what is installed.
unsafe fn se_text(app: &Rc<App>) -> String {
    match app.st.borrow().dll_version() {
        Some(v) => t!("setup-se-installed", version = v),
        None => t!("setup-se-not-installed"),
    }
}

/// Run the setup. Returns when it is closed; then starts the tour if that was chosen.
pub unsafe fn run(app: &Rc<App>) {
    let d = QDialog::new_1a(&app.window);
    d.set_window_title(&qs(t!("setup-title")));
    d.resize_2a(620, 420);
    let outer = QVBoxLayout::new_1a(&d);
    let stack = QStackedWidget::new_0a();
    outer.add_widget_2a(&stack, 1);

    // 1. Welcome and the game.
    let v = page(&stack, &t!("setup-welcome-title"), &t!("setup-welcome-text"));
    let game = label("");
    v.add_widget(&game);
    let change = QPushButton::from_q_string(&qs(t!("setup-game-change")));
    change.set_tool_tip(&qs(t!("setup-game-change-tip")));
    button_row(&v, &[&change]);
    v.add_stretch_1a(1);

    // 2. Your mods.
    let v = page(&stack, &t!("setup-mods-title"), &t!("setup-mods-text"));
    let imp_ca = QPushButton::from_q_string(&qs(t!("settings-import-ca")));
    let imp_coll = QPushButton::from_q_string(&qs(t!("settings-import-collection")));
    button_row(&v, &[&imp_ca, &imp_coll]);
    let hint = label(&t!("setup-mods-hint"));
    colored(&hint, &icons::grey());
    v.add_widget(&hint);
    v.add_stretch_1a(1);

    // 3. Script extender: installing asks first, like every install.
    let v = page(&stack, &t!("setup-se-title"), &t!("setup-se-text"));
    let se_state = label("");
    v.add_widget(&se_state);
    let se_install = QPushButton::from_q_string(&qs(t!("setup-se-install")));
    se_install.set_tool_tip(&qs(t!("setup-se-install-tip")));
    button_row(&v, &[&se_install]);
    let se_use = QCheckBox::from_q_string(&qs(t!("dlls-se-use")));
    v.add_widget(&se_use);
    v.add_stretch_1a(1);

    // 4. Updates.
    let v = page(&stack, &t!("setup-updates-title"), &t!("setup-updates-text"));
    let s = app.ctx.settings();
    let app_up = QCheckBox::from_q_string(&qs(t!("settings-app-updates")));
    app_up.set_checked(s.check_app_updates);
    let dll_up = QCheckBox::from_q_string(&qs(t!("settings-dll-updates")));
    dll_up.set_checked(s.check_dll_updates);
    let ws_up = QCheckBox::from_q_string(&qs(t!("settings-ws-updates")));
    ws_up.set_checked(s.update_workshop_on_launch);
    let tray = QCheckBox::from_q_string(&qs(t!("settings-tray")));
    tray.set_checked(s.minimize_to_tray);
    for c in [&app_up, &dll_up, &ws_up, &tray] {
        v.add_widget(c);
    }
    v.add_stretch_1a(1);

    // 5. Done.
    let v = page(&stack, &t!("setup-done-title"), &t!("setup-done-text"));
    let tour_btn = QPushButton::from_q_string(&qs(t!("setup-take-tour")));
    let start_btn = QPushButton::from_q_string(&qs(t!("setup-start")));
    button_row(&v, &[&tour_btn, &start_btn]);
    v.add_stretch_1a(1);

    let nav = QHBoxLayout::new_0a();
    outer.add_layout_1a(&nav);
    let skip = QPushButton::from_q_string(&qs(t!("setup-skip")));
    nav.add_widget(&skip);
    nav.add_stretch_1a(1);
    let step = QLabel::new();
    colored(&step, &icons::grey());
    nav.add_widget(&step);
    let back = QPushButton::from_q_string(&qs(t!("tour-back")));
    let next = QPushButton::from_q_string(&qs(t!("tour-next")));
    next.set_default(true);
    nav.add_widget(&back);
    nav.add_widget(&next);

    let take_tour = Rc::new(Cell::new(false));
    let pages = stack.count();
    // Fill the pages from the current state (the game scan may land while setup is open).
    let refresh = {
        let (a, sp, g, ss, su, bp, np, stp) = (app.clone(), stack.as_ptr(), game.as_ptr(), se_state.as_ptr(), se_use.as_ptr(), back.as_ptr(), next.as_ptr(), step.as_ptr());
        let sk = skip.as_ptr();
        Rc::new(move || {
            let (text, color) = game_text(&a);
            g.set_text(&qs(text));
            let p = qt_gui::QPalette::new_copy(g.palette());
            p.set_color_2a(qt_gui::q_palette::ColorRole::WindowText, &color);
            g.set_palette(&p);
            ss.set_text(&qs(se_text(&a)));
            su.set_checked(a.st.borrow().active().dll);
            let i = sp.current_index();
            bp.set_enabled(i > 0);
            np.set_visible(i + 1 < pages);
            sk.set_visible(i + 1 < pages);
            stp.set_text(&qs(t!("tour-count", step = i + 1, total = pages)));
        })
    };
    refresh();
    {
        let (sp, r) = (stack.as_ptr(), refresh.clone());
        next.clicked().connect(&SlotNoArgs::new(&d, move || {
            sp.set_current_index((sp.current_index() + 1).min(pages - 1));
            r();
        }));
        let (sp, r) = (stack.as_ptr(), refresh.clone());
        back.clicked().connect(&SlotNoArgs::new(&d, move || {
            sp.set_current_index((sp.current_index() - 1).max(0));
            r();
        }));
    }
    {
        let (a, r) = (app.clone(), refresh.clone());
        change.clicked().connect(&SlotNoArgs::new(&d, move || {
            a.settings.game_change.click();
            r();
        }));
        let a = app.clone();
        imp_ca.clicked().connect(&SlotNoArgs::new(&d, move || a.settings.imp_ca.click()));
        let a = app.clone();
        imp_coll.clicked().connect(&SlotNoArgs::new(&d, move || a.settings.imp_collection.click()));
        // The usual update check: it offers the newest version and asks before installing.
        let a = app.clone();
        se_install.clicked().connect(&SlotNoArgs::new(&d, move || a.settings.check_dll(&a, true)));
        let a = app.clone();
        se_use.clicked().connect(&qt_core::SlotOfBool::new(&d, move |on| {
            a.st.borrow_mut().update_active(|p| p.dll = on);
            a.mark(DIRTY_LAUNCH);
        }));
    }
    let dp = d.as_ptr();
    skip.clicked().connect(&SlotNoArgs::new(&d, move || dp.reject()));
    let dp = d.as_ptr();
    start_btn.clicked().connect(&SlotNoArgs::new(&d, move || dp.accept()));
    let (dp, tt) = (d.as_ptr(), take_tour.clone());
    tour_btn.clicked().connect(&SlotNoArgs::new(&d, move || {
        tt.set(true);
        dp.accept();
    }));
    // The SE check answers later; keep the page current while it is open.
    let timer = qt_core::QTimer::new_1a(&d);
    let r = refresh.clone();
    timer.timeout().connect(&SlotNoArgs::new(&d, move || r()));
    timer.start_1a(1000);

    d.exec();
    let mut s = app.ctx.settings();
    s.check_app_updates = app_up.is_checked();
    s.check_dll_updates = dll_up.is_checked();
    s.update_workshop_on_launch = ws_up.is_checked();
    s.minimize_to_tray = tray.is_checked();
    s.setup_done = SETUP_VERSION;
    if let Err(e) = app.ctx.set_settings(s) {
        crate::dialogs::error(&app.window, &e);
    }
    app.tray.set_visible(app.ctx.settings().minimize_to_tray);
    app.mark(DIRTY_ALL);
    d.delete_later();
    if take_tour.get() {
        crate::tour::start(app);
    }
}
