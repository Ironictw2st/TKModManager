//! Right-hand details pane for the focused mod. Rebuilt from `State` by the pump.

use crate::app::{App, DIRTY_DETAILS, DIRTY_HEADER, DIRTY_LAUNCH, DIRTY_LIST};
use crate::icons;
use qt_core::{qs, AlignmentFlag, QBox, QFlags, QUrl, SlotNoArgs, SlotOfBool, TextInteractionFlag, TransformationMode, AspectRatioMode};
use qt_gui::{q_palette::ColorRole, QColor, QDesktopServices, QPixmap};
use qt_widgets::{QCheckBox, QFrame, QHBoxLayout, QLabel, QLineEdit, QPlainTextEdit, QPushButton, QVBoxLayout, QWidget};
use std::rc::Rc;
use std::time::Instant;
use tkmm_core::fmt::{format_bytes, format_date};
use tkmm_core::packs::{ModSource, PackType};
use tkmm_core::profile_ops;
use tkmm_core::status::{self, StatusKind};

pub fn open_url(url: &str) {
    unsafe {
        QDesktopServices::open_url(&QUrl::new_1a(&qs(url)));
    }
}

/// Open Explorer with the file selected (explorer is a GUI process: no console window).
pub fn reveal(path: &str) {
    let _ = std::process::Command::new("explorer").arg(format!("/select,{path}")).spawn();
}

pub unsafe fn label(text: &str) -> QBox<QLabel> {
    let l = QLabel::from_q_string(&qs(text));
    l.set_word_wrap(true);
    l.set_text_interaction_flags(QFlags::from(TextInteractionFlag::TextSelectableByMouse));
    l
}

pub unsafe fn colored(l: &QBox<QLabel>, c: &cpp_core::CppBox<QColor>) {
    let p = qt_gui::QPalette::new_copy(l.palette());
    p.set_color_2a(ColorRole::WindowText, c);
    l.set_palette(&p);
}

pub unsafe fn bold(l: &QBox<QLabel>) {
    let f = qt_gui::QFont::new_copy(l.font());
    f.set_bold(true);
    l.set_font(&f);
}

/// A boxed section with a colored title line.
unsafe fn section(title: &str, color: Option<&cpp_core::CppBox<QColor>>) -> (QBox<QFrame>, QBox<QVBoxLayout>) {
    let frame = QFrame::new_0a();
    frame.set_frame_shape(qt_widgets::q_frame::Shape::StyledPanel);
    let l = QVBoxLayout::new_1a(&frame);
    l.set_contents_margins_4a(8, 6, 8, 6);
    l.set_spacing(3);
    let t = label(title);
    bold(&t);
    if let Some(c) = color {
        colored(&t, c);
    }
    l.add_widget(&t);
    (frame, l)
}

unsafe fn small(text: &str) -> QBox<QLabel> {
    let l = label(text);
    let f = qt_gui::QFont::new_copy(l.font());
    f.set_point_size_f(f.point_size_f() - 0.5);
    l.set_font(&f);
    l
}

pub unsafe fn refresh(app: &Rc<App>) {
    let content = QWidget::new_0a();
    let v = QVBoxLayout::new_1a(&content);
    v.set_contents_margins_4a(4, 4, 8, 4);
    v.set_spacing(8);

    let st = app.st.borrow();
    let Some(key) = st.focused.clone() else {
        v.add_widget(&label("Select a mod to see its details."));
        let legend = small(
            "The dot next to each mod shows its status: green up to date, amber last updated before the current game build, red update pending on Steam, grey local file. SE marks mods that need the script extender.",
        );
        v.add_widget(&legend);
        v.add_stretch_1a(1);
        drop(st);
        app.details_area.set_widget(&content);
        return;
    };
    if tkmm_core::groups::is_separator_key(&key) {
        let profile = st.active();
        let name = profile.entries.iter().find(|e| e.key == key).and_then(|e| e.label.clone()).unwrap_or_else(|| "Group".into());
        let members: Vec<String> = tkmm_core::groups::group_members(&profile.entries, &key).into_iter().filter(|k| st.module(k).is_some()).collect();
        let on = members.iter().filter(|k| profile.entries.iter().any(|e| &e.key == *k && e.enabled)).count();
        let t = label(&name);
        bold(&t);
        v.add_widget(&t);
        v.add_widget(&label(&format!("Group · {} mod{}, {on} enabled", members.len(), if members.len() == 1 { "" } else { "s" })));
        v.add_widget(&small("Every mod below this group header, up to the next group, belongs to it. Tick the header to turn the whole group on or off, drag it to move all of its mods, and right-click it to rename or remove it (its mods stay)."));
        v.add_stretch_1a(1);
        drop(st);
        app.details_area.set_widget(&content);
        return;
    }
    let Some(m) = st.module(&key).cloned() else {
        let t = label("Missing pack");
        bold(&t);
        v.add_widget(&t);
        v.add_widget(&small(&key));
        v.add_widget(&small("This profile entry points at a pack that is no longer installed (unsubscribed or deleted). It is skipped at launch."));
        v.add_stretch_1a(1);
        drop(st);
        app.details_area.set_widget(&content);
        return;
    };
    let ws = st.ws_of(&m).cloned();
    let meta = st.meta_of(&key);
    let profile = st.active();
    let entry = profile.entries.iter().find(|e| e.key == key).cloned();
    let cutoff = st.cutoff(app.ctx.settings().outdated_before);
    let have = st.dll_version();

    if let Some(p) = &m.preview_path {
        let pm = QPixmap::from_q_string(&qs(p));
        if !pm.is_null() {
            let img = QLabel::new();
            img.set_pixmap(&pm.scaled_2_int_aspect_ratio_mode_transformation_mode(300, 170, AspectRatioMode::KeepAspectRatio, TransformationMode::SmoothTransformation));
            img.set_alignment(QFlags::from(AlignmentFlag::AlignHCenter));
            v.add_widget(&img);
        }
    }

    let title = label(&st.title_of(&key));
    let f = qt_gui::QFont::new_copy(title.font());
    f.set_bold(true);
    f.set_point_size(f.point_size() + 1);
    title.set_font(&f);
    v.add_widget(&title);
    v.add_widget(&small(&m.file));

    // Status.
    let stat = status::mod_status(Some(&m), ws.as_ref(), cutoff);
    let color = match stat.kind {
        StatusKind::Unknown | StatusKind::Local => None,
        k => Some(icons::status_color(k)),
    };
    let (frame, l) = section(stat.kind.label(), color.as_ref());
    l.add_widget(&small(&stat.text));
    v.add_widget(&frame);

    // Script extender.
    let req = st.se_req(&key);
    let unmet = status::se_unmet(&req, profile.dll, have.as_deref());
    let enabled = entry.as_ref().map(|e| e.enabled).unwrap_or(false);
    let se_color = if req.required { Some(if unmet.is_some() && enabled { icons::red() } else { icons::blue() }) } else { None };
    let (frame, l) = section(if req.required { "Needs the script extender" } else { "No script extender needed" }, se_color.as_ref());
    l.add_widget(&small(&status::source_text(&req)));
    if let Some(n) = &req.notes {
        l.add_widget(&label(n));
    }
    if let Some(e) = &req.error {
        let w = small(e);
        colored(&w, &icons::amber());
        l.add_widget(&w);
    }
    if let Some(u) = unmet {
        let text = status::unmet_text(u, &req, have.as_deref());
        let w = small(&if enabled { text } else { format!("{text} (applies once this mod is enabled)") });
        if enabled {
            colored(&w, &icons::red());
        }
        l.add_widget(&w);
    }
    v.add_widget(&frame);

    if m.pack_type == PackType::Movie {
        let w = small("Movie packs load after every mod pack and cannot be ordered. Enabled = loaded; disabled = excluded through the mod list. Files are never renamed or moved.");
        colored(&w, &icons::amber());
        v.add_widget(&w);
    }

    let facts = vec![
        match m.source {
            ModSource::Workshop => "Workshop".to_string(),
            ModSource::Data => "data/".to_string(),
            ModSource::Folder => "Extra folder".to_string(),
        },
        if m.pack_type == PackType::Movie { "movie pack".into() } else { "mod pack".into() },
        format_bytes(m.size),
        format!("file {}", format_date(m.mtime)),
    ];
    v.add_widget(&small(&facts.join(" · ")));

    let buttons = QHBoxLayout::new_0a();
    v.add_layout_1a(&buttons);
    if let Some(id) = m.workshop_id.clone() {
        let b = QPushButton::from_q_string(&qs("Open in Workshop"));
        b.clicked().connect(&SlotNoArgs::new(&b, move || open_url(&format!("https://steamcommunity.com/sharedfiles/filedetails/?id={id}"))));
        buttons.add_widget(&b);
    }
    let b = QPushButton::from_q_string(&qs("Show file"));
    let path = m.path.clone();
    b.clicked().connect(&SlotNoArgs::new(&b, move || reveal(&path)));
    buttons.add_widget(&b);
    buttons.add_stretch_1a(1);

    // Required items.
    let required: Vec<(String, Option<String>, bool, String)> = ws
        .as_ref()
        .map(|w| {
            w.required_items
                .iter()
                .map(|id| {
                    let installed = st.mods.iter().find(|x| x.workshop_id.as_deref() == Some(id.as_str()));
                    let on = installed.map(|x| profile.entries.iter().any(|e| e.key == x.key && e.enabled)).unwrap_or(false);
                    let title = st.workshop.get(id).map(|w| w.title.clone()).filter(|t| !t.is_empty()).or(installed.map(|x| x.file.clone())).unwrap_or_else(|| id.clone());
                    (id.clone(), installed.map(|x| x.key.clone()), on, title)
                })
                .collect()
        })
        .unwrap_or_default();
    if !required.is_empty() {
        let (frame, l) = section("Required items", None);
        for (id, key, on, title) in &required {
            let row = QHBoxLayout::new_0a();
            l.add_layout_1a(&row);
            let (state, c) = match (key, on) {
                (None, _) => ("missing", icons::red()),
                (Some(_), true) => ("on", icons::green()),
                (Some(_), false) => ("off", icons::amber()),
            };
            let s = small(state);
            colored(&s, &c);
            s.set_fixed_width(48);
            row.add_widget(&s);
            let link = QPushButton::from_q_string(&qs(title));
            link.set_flat(true);
            let id = id.clone();
            link.clicked().connect(&SlotNoArgs::new(&link, move || open_url(&format!("https://steamcommunity.com/sharedfiles/filedetails/?id={id}"))));
            row.add_widget_2a(&link, 1);
        }
        let off: Vec<String> = required.iter().filter(|(_, k, on, _)| k.is_some() && !on).filter_map(|(_, k, _, _)| k.clone()).collect();
        if !off.is_empty() {
            let b = QPushButton::from_q_string(&qs("Enable all required items"));
            let this = app.clone();
            b.clicked().connect(&SlotNoArgs::new(&b, move || {
                this.st.borrow_mut().update_active(|p| profile_ops::toggle(&mut p.entries, &off, Some(true)));
                this.mark(DIRTY_LIST | DIRTY_DETAILS | DIRTY_LAUNCH | DIRTY_HEADER);
            }));
            l.add_widget(&b);
        }
        v.add_widget(&frame);
    }

    // Tags, hidden, notes.
    let (frame, l) = section("Tags", None);
    let tag_row = QHBoxLayout::new_0a();
    l.add_layout_1a(&tag_row);
    for t in &meta.tags {
        let b = QPushButton::from_q_string(&qs(format!("{t}  ✕")));
        b.set_tool_tip(&qs("Remove tag"));
        let (this, k, t) = (app.clone(), key.clone(), t.clone());
        b.clicked().connect(&SlotNoArgs::new(&b, move || {
            this.st.borrow_mut().set_meta(&k, |m| m.tags.retain(|x| x != &t));
            this.mark(DIRTY_DETAILS | DIRTY_LIST);
        }));
        tag_row.add_widget(&b);
    }
    tag_row.add_stretch_1a(1);
    let add_row = QHBoxLayout::new_0a();
    l.add_layout_1a(&add_row);
    let input = QLineEdit::new();
    input.set_placeholder_text(&qs("Add tag…"));
    let add = QPushButton::from_q_string(&qs("Add"));
    add_row.add_widget_2a(&input, 1);
    add_row.add_widget(&add);
    let (this, k, ip) = (app.clone(), key.clone(), input.as_ptr());
    let add_tag = SlotNoArgs::new(&add, move || {
        let t = ip.text().to_std_string().trim().to_string();
        if t.is_empty() {
            return;
        }
        this.st.borrow_mut().set_meta(&k, |m| {
            if !m.tags.contains(&t) {
                m.tags.push(t.clone());
            }
        });
        this.mark(DIRTY_DETAILS | DIRTY_LIST);
    });
    add.clicked().connect(&add_tag);
    input.return_pressed().connect(&add_tag);
    let hidden = QCheckBox::from_q_string(&qs("Hidden (only shown with \"Show hidden\")"));
    hidden.set_checked(meta.hidden);
    let (this, k) = (app.clone(), key.clone());
    hidden.clicked().connect(&SlotOfBool::new(&hidden, move |on| {
        this.st.borrow_mut().set_meta(&k, |m| m.hidden = on);
        this.mark(DIRTY_LIST);
    }));
    l.add_widget(&hidden);
    v.add_widget(&frame);

    let (frame, l) = section("Notes", None);
    let notes = QPlainTextEdit::new();
    notes.set_plain_text(&qs(&meta.notes));
    notes.set_placeholder_text(&qs("Your notes for this mod"));
    notes.set_maximum_height(90);
    let (this, k, np) = (app.clone(), key.clone(), notes.as_ptr());
    notes.text_changed().connect(&SlotNoArgs::new(&notes, move || {
        *this.notes_pending.borrow_mut() = Some((k.clone(), np.to_plain_text().to_std_string(), Instant::now()));
    }));
    l.add_widget(&notes);
    v.add_widget(&frame);

    if let Some(desc) = ws.as_ref().map(|w| strip_bbcode(&w.description)).filter(|d| !d.is_empty()) {
        let (frame, l) = section("Description", None);
        l.add_widget(&small(&desc));
        v.add_widget(&frame);
    }
    v.add_stretch_1a(1);
    drop(st);
    app.details_area.set_widget(&content);
}

/// Steam descriptions are BBCode; strip tags for plain text.
pub fn strip_bbcode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_img = false;
    let mut i = 0;
    let b = s.as_bytes();
    while i < b.len() {
        if b[i] == b'[' {
            if let Some(end) = s[i..].find(']') {
                let tag = s[i + 1..i + end].to_ascii_lowercase();
                if tag == "img" {
                    in_img = true;
                } else if tag == "/img" {
                    in_img = false;
                }
                i += end + 1;
                continue;
            }
        }
        let ch = s[i..].chars().next().unwrap();
        if !in_img {
            out.push(ch);
        }
        i += ch.len_utf8();
    }
    let mut text = out.replace("\r\n", "\n");
    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    text.trim().chars().take(3000).collect()
}
