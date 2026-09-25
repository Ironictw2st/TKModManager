//! Bottom of the right column (mockup order): Play, Script extender, Skip intro, Load save,
//! plus script-extender warnings, the crash notice and the launch status line.

use crate::app::{App, DIRTY_LAUNCH, DIRTY_LIST, DIRTY_DETAILS, PAGE_LOGS, PAGE_SETTINGS};
use crate::details::{bold, colored, label};
use crate::icons;
use qt_core::{qs, QVariant, SlotNoArgs, SlotOfBool, SlotOfInt};
use qt_widgets::{QCheckBox, QComboBox, QFrame, QHBoxLayout, QLabel, QLayoutItem, QPushButton, QVBoxLayout, QWidget};
use std::rc::Rc;
use tkmm_core::fmt::format_date;
use tkmm_core::status::{self, SeUnmet};

unsafe fn clear_layout(l: &qt_core::QBox<QVBoxLayout>) {
    loop {
        let it: cpp_core::CppBox<QLayoutItem> = {
            let p = l.take_at(0);
            if p.is_null() {
                break;
            }
            cpp_core::CppBox::from_raw(p.as_mut_raw_ptr()).unwrap()
        };
        let w = it.widget();
        if !w.is_null() {
            w.delete_later();
        }
    }
}

/// Wrapped labels here don't get their height from the nested layouts, so size them from the
/// column's real width.
unsafe fn fit(l: &qt_core::QBox<QLabel>, width: i32) {
    l.set_minimum_height(l.height_for_width(width));
}

pub unsafe fn refresh(app: &Rc<App>) {
    clear_layout(&app.launch_layout);
    let width = { let w = app.launch_layout.parent_widget().width(); if w > 100 { w } else { 290 } } - 18;
    let panel = QWidget::new_0a();
    let v = QVBoxLayout::new_1a(&panel);
    v.set_contents_margins_4a(0, 6, 0, 0);
    v.set_spacing(6);

    let st = app.st.borrow();
    let profile = st.active();
    let running = st.game_running;
    let have = st.dll_version();
    let enabled = st.enabled_count();

    let line = QFrame::new_0a();
    line.set_frame_shape(qt_widgets::q_frame::Shape::HLine);
    v.add_widget(&line);

    let waiting = app.launch_after_steam.get();
    let play = QPushButton::from_q_string(&qs(if running {
        "Running…"
    } else if waiting {
        "▶  Play without waiting"
    } else {
        "▶  Play"
    }));
    play.set_minimum_height(40);
    let f = qt_gui::QFont::new_copy(play.font());
    f.set_bold(true);
    f.set_point_size(f.point_size() + 3);
    play.set_font(&f);
    play.set_default(true);
    play.set_enabled(!running && st.paths.game_root.is_some());
    play.set_tool_tip(&qs(if running {
        "The game is already running".to_string()
    } else if waiting {
        "Steam is still updating Workshop mods; start now and some may load their old version".to_string()
    } else {
        format!("Launch {} with {enabled} mods", profile.name)
    }));
    let this = app.clone();
    play.clicked().connect(&SlotNoArgs::new(&play, move || this.launch(None)));
    v.add_widget(&play);
    let summary = label(&format!("{} · {enabled} mods enabled", profile.name));
    summary.set_alignment(qt_core::QFlags::from(qt_core::AlignmentFlag::AlignHCenter));
    v.add_widget(&summary);

    // More than one copy of the game (Steam and Epic, say): switch here instead of Settings.
    if st.installs.len() > 1 {
        let r = QHBoxLayout::new_0a();
        v.add_layout_1a(&r);
        r.add_widget(QLabel::from_q_string(&qs("Copy")).into_ptr());
        let copies = QComboBox::new_0a();
        let current = st.paths.game_root.clone().unwrap_or_default();
        for i in &st.installs {
            copies.add_item_q_string_q_variant(&qs(i.store.label()), &QVariant::from_q_string(&qs(&i.root)));
            copies.set_item_data_3a(copies.count() - 1, &QVariant::from_q_string(&qs(&i.root)), qt_core::ItemDataRole::ToolTipRole.to_int());
        }
        let ci = copies.find_data_1a(&QVariant::from_q_string(&qs(&current)));
        copies.set_current_index(ci.max(0));
        copies.set_enabled(!running);
        let (this, cp) = (app.clone(), copies.as_ptr());
        copies.activated().connect(&SlotOfInt::new(&copies, move |_| {
            let root = cp.current_data_0a().to_string().to_std_string();
            if root.is_empty() || this.st.borrow().paths.game_root.as_deref() == Some(root.as_str()) {
                return;
            }
            this.set_game_root(&root);
        }));
        r.add_widget_2a(&copies, 1);
    }

    // Script-extender problems for this launch.
    let problems = status::se_problems(&profile, |k| st.se_req(k), |k| st.title_of(k), have.as_deref());
    for p in &problems {
        let frame = QFrame::new_0a();
        frame.set_frame_shape(qt_widgets::q_frame::Shape::StyledPanel);
        let l = QVBoxLayout::new_1a(&frame);
        l.set_contents_margins_4a(8, 6, 8, 6);
        let head = label(&match p.kind {
            SeUnmet::Off => format!("{} enabled mod{} need{} the script extender.", p.mods.len(), if p.mods.len() > 1 { "s" } else { "" }, if p.mods.len() > 1 { "" } else { "s" }),
            _ => p.text.clone(),
        });
        colored(&head, &if p.kind == SeUnmet::Off { icons::amber() } else { icons::red() });
        bold(&head);
        fit(&head, width);
        l.add_widget(&head);
        let names = label(&{
            let mut s = p.mods.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            if p.mods.len() > 3 {
                s.push_str(&format!(" +{} more", p.mods.len() - 3));
            }
            s
        });
        names.set_tool_tip(&qs(p.mods.join("\n")));
        fit(&names, width);
        l.add_widget(&names);
        let b = if p.kind == SeUnmet::Off { QPushButton::from_q_string(&qs("Turn on script extender")) } else { QPushButton::from_q_string(&qs("Script extender settings")) };
        let (this, kind) = (app.clone(), p.kind);
        b.set_enabled(!running);
        b.clicked().connect(&SlotNoArgs::new(&b, move || {
            if kind == SeUnmet::Off {
                this.st.borrow_mut().update_active(|p| p.dll = true);
                this.mark(DIRTY_LAUNCH | DIRTY_LIST | DIRTY_DETAILS);
            } else {
                this.set_page(PAGE_SETTINGS);
            }
        }));
        l.add_widget(&b);
        v.add_widget(&frame);
    }

    // Options.
    let se_row = QHBoxLayout::new_0a();
    v.add_layout_1a(&se_row);
    let se = QCheckBox::from_q_string(&qs("Script extender"));
    se.set_checked(profile.dll);
    se.set_enabled(!running);
    se.set_tool_tip(&qs("Inject the script extender DLL once the main menu is up"));
    let this = app.clone();
    se.clicked().connect(&SlotOfBool::new(&se, move |on| {
        this.st.borrow_mut().update_active(|p| p.dll = on);
        this.mark(DIRTY_LAUNCH | DIRTY_LIST | DIRTY_DETAILS);
    }));
    se_row.add_widget(&se);
    se_row.add_stretch_1a(1);
    let ver = QPushButton::from_q_string(&qs(have.as_ref().map(|v| format!("v{v}")).unwrap_or_else(|| "no DLL".into())));
    ver.set_flat(true);
    ver.set_tool_tip(&qs(if have.is_some() { "DLL that matches this game build" } else { "No DLL matching this game build is installed" }));
    let this = app.clone();
    ver.clicked().connect(&SlotNoArgs::new(&ver, move || this.set_page(PAGE_SETTINGS)));
    se_row.add_widget(&ver);

    let skip = QCheckBox::from_q_string(&qs("Skip intro"));
    skip.set_checked(profile.skip_intro);
    skip.set_enabled(!running);
    skip.set_tool_tip(&qs("Skip the two startup videos (generated options pack)"));
    let this = app.clone();
    skip.clicked().connect(&SlotOfBool::new(&skip, move |on| {
        this.st.borrow_mut().update_active(|p| p.skip_intro = on);
    }));
    v.add_widget(&skip);

    let save_row = QHBoxLayout::new_0a();
    v.add_layout_1a(&save_row);
    save_row.add_widget(QLabel::from_q_string(&qs("Load save")).into_ptr());
    let saves = QComboBox::new_0a();
    saves.add_item_q_string_q_variant(&qs(if st.saves.is_empty() { "(no saves)" } else { "(main menu)" }), &QVariant::from_q_string(&qs("")));
    for s in st.saves.iter().take(40) {
        saves.add_item_q_string_q_variant(&qs(format!("{} · {}", s.name, format_date(s.mtime))), &QVariant::from_q_string(&qs(&s.name)));
    }
    let cur = saves.find_data_1a(&QVariant::from_q_string(&qs(&st.load_save)));
    saves.set_current_index(cur.max(0));
    saves.set_enabled(!running && !st.saves.is_empty());
    saves.set_tool_tip(&qs("Load this campaign save straight away"));
    let (this, sp) = (app.clone(), saves.as_ptr());
    saves.activated().connect(&SlotOfInt::new(&saves, move |_| {
        this.st.borrow_mut().load_save = sp.current_data_0a().to_string().to_std_string();
    }));
    save_row.add_widget_2a(&saves, 1);

    if let Some((id, code)) = st.crash {
        let l = label(&format!("Game ended abnormally (exit code {})", code.map(|c| format!("0x{c:X}")).unwrap_or_else(|| "?".into())));
        colored(&l, &icons::red());
        fit(&l, width);
        v.add_widget(&l);
        let row = QHBoxLayout::new_0a();
        v.add_layout_1a(&row);
        let b = QPushButton::from_q_string(&qs("Details"));
        let this = app.clone();
        b.clicked().connect(&SlotNoArgs::new(&b, move || crate::tabs::crash_dialog(&this, id)));
        row.add_widget(&b);
        let r = QPushButton::from_q_string(&qs("Mod report"));
        r.set_tool_tip(&qs("Write a report of every loaded mod, the game's crash files and the logs to the Desktop, to send to a mod author"));
        let this = app.clone();
        r.clicked().connect(&SlotNoArgs::new(&r, move || this.create_report()));
        row.add_widget(&r);
        if crate::jobs::REPORT_WEBHOOK.is_some() {
            let s = QPushButton::from_q_string(&qs("Send report"));
            s.set_tool_tip(&qs("Post the mod report and the crash dump to the 190 Expanded Discord"));
            let this = app.clone();
            s.clicked().connect(&SlotNoArgs::new(&s, move || this.send_report()));
            row.add_widget(&s);
        }
        let x = QPushButton::from_q_string(&qs("Dismiss"));
        let this = app.clone();
        x.clicked().connect(&SlotNoArgs::new(&x, move || {
            this.st.borrow_mut().crash = None;
            this.mark(DIRTY_LAUNCH);
        }));
        row.add_widget(&x);
        row.add_stretch_1a(1);
    }

    // The crash row above already says how the game ended.
    if let Some(s) = st.launch.as_ref().filter(|s| !(s.phase == "crashed" && st.crash.is_some())) {
        let b = QPushButton::from_q_string(&qs(&s.message));
        b.set_flat(true);
        b.set_tool_tip(&qs(format!("{}{} (open the Logs tab)", s.message, s.pid.map(|p| format!(" · pid {p}")).unwrap_or_default())));
        let c = match s.phase.as_str() {
            "verified" => Some(icons::green()),
            "failed" | "mismatch" | "crashed" => Some(icons::red()),
            "spawned" | "menu" | "injected" => Some(icons::amber()),
            _ => None,
        };
        if let Some(c) = c {
            let p = qt_gui::QPalette::new_copy(b.palette());
            p.set_color_2a(qt_gui::q_palette::ColorRole::ButtonText, &c);
            b.set_palette(&p);
        }
        let this = app.clone();
        b.clicked().connect(&SlotNoArgs::new(&b, move || this.set_page(PAGE_LOGS)));
        v.add_widget(&b);
    }

    let foot = label(&format!(
        "{} · TK Mod Manager v{}",
        st.paths.game_root.as_ref().map(|_| format!("Three Kingdoms {}", st.paths.exe_version.clone().unwrap_or_default())).unwrap_or_else(|| "game not found".into()),
        tkmm_core::ops::APP_VERSION
    ));
    foot.set_alignment(qt_core::QFlags::from(qt_core::AlignmentFlag::AlignHCenter));
    let ff = qt_gui::QFont::new_copy(foot.font());
    ff.set_point_size_f(ff.point_size_f() - 1.0);
    foot.set_font(&ff);
    colored(&foot, &icons::grey());
    v.add_widget(&foot);
    drop(st);
    app.launch_layout.add_widget(&panel);
}
