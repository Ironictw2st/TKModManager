//! Script-extender settings dialog: every `script_extender.cfg` key as a control, grouped as
//! "Multiplayer: every player must match" / "Local" / "Main-menu text", plus a raw text editor
//! for keys the manager does not know. Used for a profile's own settings and for the default.

use crate::app::{App, DIRTY_LAUNCH};
use crate::details::label;
use crate::icons;
use qt_core::{qs, QBox, QVariant, SlotNoArgs, SlotOfBool, SlotOfInt, SlotOfQString};
use qt_widgets::{
    q_dialog_button_box::ButtonRole, q_dialog_button_box::StandardButton as BoxButton, QCheckBox, QComboBox, QDialog, QDialogButtonBox, QGridLayout,
    QGroupBox, QLabel, QLineEdit, QPushButton, QScrollArea, QSpinBox, QVBoxLayout, QWidget,
};
use std::cell::RefCell;
use std::rc::Rc;
use tkmm_core::dll;
use tkmm_core::se_config::{Group, Kind, SeConfig, KEYS};

enum Ctrl {
    Check(QBox<QCheckBox>),
    Spin(QBox<QSpinBox>),
    Combo(QBox<QComboBox>),
    Line(QBox<QLineEdit>),
}

impl Ctrl {
    unsafe fn get(&self) -> String {
        match self {
            Ctrl::Check(c) => if c.is_checked() { "1" } else { "0" }.to_string(),
            Ctrl::Spin(s) => s.value().to_string(),
            Ctrl::Combo(c) => c.current_data_0a().to_string().to_std_string(),
            Ctrl::Line(l) => l.text().to_std_string(),
        }
    }

    unsafe fn set(&self, v: &str) {
        match self {
            Ctrl::Check(c) => c.set_checked(v == "1"),
            Ctrl::Spin(s) => s.set_value(v.parse().unwrap_or(0)),
            Ctrl::Combo(c) => c.set_current_index(c.find_data_1a(&QVariant::from_q_string(&qs(v))).max(0)),
            Ctrl::Line(l) => l.set_text(&qs(v)),
        }
    }

    unsafe fn widget(&self) -> cpp_core::Ptr<QWidget> {
        match self {
            Ctrl::Check(c) => c.as_ptr().static_upcast(),
            Ctrl::Spin(s) => s.as_ptr().static_upcast(),
            Ctrl::Combo(c) => c.as_ptr().static_upcast(),
            Ctrl::Line(l) => l.as_ptr().static_upcast(),
        }
    }
}

struct Row {
    key: &'static str,
    default: &'static str,
    caption: QBox<QLabel>,
    ctrl: Ctrl,
}

struct Form {
    rows: Vec<Row>,
    /// The config being edited; its unknown keys survive the form.
    base: RefCell<SeConfig>,
    other: QBox<QLabel>,
}

impl Form {
    unsafe fn load(&self, c: &SeConfig) {
        *self.base.borrow_mut() = c.clone();
        for r in &self.rows {
            r.ctrl.set(&c.get(r.key));
        }
        self.refresh();
    }

    unsafe fn read(&self) -> SeConfig {
        let mut c = self.base.borrow().clone();
        for r in &self.rows {
            c.set(r.key, &r.ctrl.get());
        }
        c
    }

    /// Changed values in bold; the list of keys only the text editor shows.
    unsafe fn refresh(&self) {
        for r in &self.rows {
            let f = qt_gui::QFont::new_copy(r.caption.font());
            f.set_bold(r.ctrl.get() != r.default);
            r.caption.set_font(&f);
        }
        let other: Vec<String> = self.base.borrow().unknown().map(|(k, v)| format!("{k}={v}")).collect();
        self.other.set_visible(!other.is_empty());
        self.other.set_text(&qs(format!("Other keys, kept as they are (edit them with \"Edit as text…\"): {}", other.join(", "))));
    }
}

/// What the dialog is editing.
pub enum Target {
    /// The default for profiles without their own settings.
    Default,
    /// The active profile.
    Profile,
}

/// Open the editor for `target` and save what the user chose.
pub unsafe fn open(app: &Rc<App>, target: Target) {
    let settings = app.ctx.settings();
    let default = dll::default_se_config(&settings);
    let (profile_name, own) = {
        let st = app.st.borrow();
        let p = st.active();
        (p.name.clone(), p.se_config.clone())
    };
    let profile_mode = matches!(target, Target::Profile);

    let d = QDialog::new_1a(&app.window);
    d.set_window_title(&qs(if profile_mode { format!("Script extender settings: {profile_name}") } else { "Default script extender settings".to_string() }));
    d.resize_2a(720, 680);
    let l = QVBoxLayout::new_1a(&d);
    l.add_widget(&label(if profile_mode {
        "Settings the script extender reads when this profile starts the game (script_extender.cfg). Changes apply at the next launch."
    } else {
        "Used by every profile that has no settings of its own. Changes apply at the next launch."
    }));
    let use_default = QCheckBox::from_q_string(&qs("Use the default settings (Settings > Script extender)"));
    use_default.set_visible(profile_mode);
    use_default.set_checked(profile_mode && own.is_none());
    l.add_widget(&use_default);

    let scroll = QScrollArea::new_0a();
    scroll.set_widget_resizable(true);
    l.add_widget_2a(&scroll, 1);
    let body = QWidget::new_0a();
    let bl = QVBoxLayout::new_1a(&body);
    let mut rows = Vec::new();
    for (group, title, note) in [
        (Group::Sim, "Multiplayer: every player must match", "These change the campaign itself. Every value is part of the version lock in the main-menu build text, so players with different values cannot see each other's lobbies. Profile export carries them, and the co-op sync check compares them."),
        (Group::Local, "Local", "Performance and diagnostics on this PC only; may differ between players."),
        (Group::Menu, "Main-menu text", "Placeholders: {game} = the game's own text, {version} = script extender version, {sync} = multiplayer settings hash. The [se version.sync] tag is always added."),
    ] {
        let g = QGroupBox::from_q_string(&qs(title));
        bl.add_widget(&g);
        let gl = QGridLayout::new_1a(&g);
        gl.set_column_stretch(1, 1);
        let n = label(note);
        crate::details::colored(&n, &icons::grey());
        gl.add_widget_5a(&n, 0, 0, 1, 2);
        let mut row = 1;
        for k in KEYS.iter().filter(|k| k.group == group) {
            let caption = QLabel::from_q_string(&qs(k.label));
            let ctrl = match k.kind {
                Kind::Bool => Ctrl::Check(QCheckBox::new()),
                Kind::Int { min, max } => {
                    let s = QSpinBox::new_0a();
                    s.set_range(min as i32, max as i32);
                    Ctrl::Spin(s)
                }
                Kind::Choice(opts) => {
                    let c = QComboBox::new_0a();
                    for (v, t) in opts {
                        c.add_item_q_string_q_variant(&qs(format!("{t} ({v})")), &QVariant::from_q_string(&qs(*v)));
                    }
                    Ctrl::Combo(c)
                }
                Kind::Text => {
                    let e = QLineEdit::new();
                    e.set_placeholder_text(&qs(k.default));
                    Ctrl::Line(e)
                }
            };
            let tip = qs(format!("{}\n\n{}  (default {})", k.help, k.key, k.default));
            caption.set_tool_tip(&tip);
            ctrl.widget().set_tool_tip(&tip);
            gl.add_widget_3a(&caption, row, 0);
            gl.add_widget_3a(ctrl.widget(), row, 1);
            row += 1;
            rows.push(Row { key: k.key, default: k.default, caption, ctrl });
        }
    }
    let other = label("");
    crate::details::colored(&other, &icons::amber());
    bl.add_widget(&other);
    bl.add_stretch_1a(1);
    scroll.set_widget(&body);

    let form = Rc::new(Form { rows, base: RefCell::new(SeConfig::default()), other });
    form.load(own.as_ref().unwrap_or(&default));
    for r in &form.rows {
        let f = form.clone();
        match &r.ctrl {
            Ctrl::Check(c) => c.toggled().connect(&SlotOfBool::new(&d, move |_| f.refresh())),
            Ctrl::Spin(s) => s.value_changed().connect(&SlotOfInt::new(&d, move |_| f.refresh())),
            Ctrl::Combo(c) => c.current_index_changed().connect(&SlotOfInt::new(&d, move |_| f.refresh())),
            Ctrl::Line(e) => e.text_changed().connect(&SlotOfQString::new(&d, move |_| f.refresh())),
        };
    }
    // "Use the default" shows the default, read-only; unticking starts from it.
    let sync_enabled = {
        let (sp, ud) = (scroll.as_ptr(), use_default.as_ptr());
        move || sp.set_enabled(!ud.is_checked())
    };
    sync_enabled();
    {
        let (f, def) = (form.clone(), default.clone());
        let se = sync_enabled.clone();
        use_default.toggled().connect(&SlotOfBool::new(&d, move |on| {
            if on {
                f.load(&def);
            }
            se();
        }));
    }

    let buttons = QDialogButtonBox::new();
    let reset = QPushButton::from_q_string(&qs("Reset to defaults"));
    let as_text = QPushButton::from_q_string(&qs("Edit as text…"));
    buttons.add_button_q_abstract_button_button_role(&reset, ButtonRole::ResetRole);
    buttons.add_button_q_abstract_button_button_role(&as_text, ButtonRole::ActionRole);
    let save = buttons.add_button_standard_button(BoxButton::Save);
    let cancel = buttons.add_button_standard_button(BoxButton::Cancel);
    l.add_widget(&buttons);
    {
        let (f, ud) = (form.clone(), use_default.as_ptr());
        reset.clicked().connect(&SlotNoArgs::new(&d, move || {
            ud.set_checked(false);
            // Unknown keys go too: "defaults" means the file as the DLL ships it.
            f.load(&SeConfig::default());
        }));
    }
    {
        let (f, ud, a) = (form.clone(), use_default.as_ptr(), app.clone());
        as_text.clicked().connect(&SlotNoArgs::new(&d, move || {
            let text = f.read().render();
            if let Some(t) = crate::dialogs::edit_text(&a.window, "script_extender.cfg", "One key=value per line; # starts a comment. Keys the manager does not know are kept.", &text) {
                ud.set_checked(false);
                f.load(&SeConfig::parse(&t));
            }
        }));
    }
    let dp = d.as_ptr();
    save.clicked().connect(&SlotNoArgs::new(&d, move || dp.accept()));
    let dp = d.as_ptr();
    cancel.clicked().connect(&SlotNoArgs::new(&d, move || dp.reject()));
    if d.exec() != 1 {
        return;
    }

    let chosen = form.read();
    if profile_mode {
        let value = (!use_default.is_checked()).then_some(chosen);
        app.st.borrow_mut().update_active(|p| p.se_config = value);
    } else {
        let mut s = app.ctx.settings();
        s.se_config_default = Some(chosen);
        if let Err(e) = app.ctx.set_settings(s) {
            crate::dialogs::error(&app.window, &e);
        }
    }
    app.mark(DIRTY_LAUNCH);
}
