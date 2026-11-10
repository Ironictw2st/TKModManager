//! The script extender's features, inline in the DLLs tab: every `script_extender.cfg` key as a
//! control, grouped by what it is about (battles, campaign AI, ...), plus a raw text editor for
//! keys the manager does not know. Edits save at once, to the active profile's own settings or
//! to the default for profiles without their own; the cfg is written at the next launch.

use crate::app::{App, DIRTY_LAUNCH};
use crate::details::{colored, label};
use crate::icons;
use cpp_core::Ptr;
use qt_core::{qs, QBox, QVariant, SlotNoArgs, SlotOfBool, SlotOfInt};
use qt_widgets::{QCheckBox, QComboBox, QGridLayout, QGroupBox, QHBoxLayout, QLabel, QLineEdit, QPushButton, QSpinBox, QVBoxLayout, QWidget};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tkmm_core::dll;
use tkmm_core::se_config::{Area, Group, Kind, SeConfig, KEYS};

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
            // Leave a field being typed in alone; it saves when editing finishes.
            Ctrl::Line(l) if !l.has_focus() => l.set_text(&qs(v)),
            Ctrl::Line(_) => {}
        }
    }

    unsafe fn widget(&self) -> Ptr<QWidget> {
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
    /// Shown in bold when the value is not the default: the caption, or the checkbox itself.
    name: Ptr<QWidget>,
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
            let f = qt_gui::QFont::new_copy(r.name.font());
            f.set_bold(r.ctrl.get() != r.default);
            r.name.set_font(&f);
        }
        let other: Vec<String> = self.base.borrow().unknown().map(|(k, v)| format!("{k}={v}")).collect();
        self.other.set_visible(!other.is_empty());
        self.other.set_text(&qs(t!("se-ui-other-keys", keys = other.join(", "))));
    }
}

/// Areas behind "Show advanced features": tuning and tools most players never need.
const ADVANCED: [Area; 4] = [Area::Performance, Area::Interface, Area::Diagnostics, Area::MenuText];

/// Which settings the panel edits (the combo's item data).
const TARGET_PROFILE: i32 = 0;
const TARGET_DEFAULT: i32 = 1;

pub struct Features {
    pub root: QBox<QGroupBox>,
    target: QBox<QComboBox>,
    use_default: QBox<QCheckBox>,
    reset: QBox<QPushButton>,
    as_text: QBox<QPushButton>,
    summary: QBox<QLabel>,
    body: QBox<QWidget>,
    show_advanced: QBox<QCheckBox>,
    advanced: QBox<QWidget>,
    form: Rc<Form>,
    /// Set while `load` fills the controls: their signals must not save.
    loading: Rc<Cell<bool>>,
}

unsafe fn small_grey(text: &str) -> QBox<QLabel> {
    let l = label(text);
    let f = qt_gui::QFont::new_copy(l.font());
    f.set_point_size_f(f.point_size_f() - 0.5);
    l.set_font(&f);
    colored(&l, &icons::grey());
    l
}

impl Features {
    pub fn build() -> Features {
        unsafe {
            let root = QGroupBox::from_q_string(&qs(t!("dlls-se-features")));
            let v = QVBoxLayout::new_1a(&root);
            let top = QHBoxLayout::new_0a();
            v.add_layout_1a(&top);
            top.add_widget(QLabel::from_q_string(&qs(t!("dlls-se-target"))).into_ptr());
            let target = QComboBox::new_0a();
            target.add_item_q_string_q_variant(&qs(t!("dlls-se-target-profile", profile = "")), &QVariant::from_int(TARGET_PROFILE));
            target.add_item_q_string_q_variant(&qs(t!("dlls-se-target-default")), &QVariant::from_int(TARGET_DEFAULT));
            target.set_tool_tip(&qs(t!("dlls-se-target-tip")));
            top.add_widget(&target);
            top.add_stretch_1a(1);
            let reset = QPushButton::from_q_string(&qs(t!("se-ui-reset")));
            let as_text = QPushButton::from_q_string(&qs(t!("se-ui-edit-text")));
            top.add_widget(&reset);
            top.add_widget(&as_text);
            let use_default = QCheckBox::from_q_string(&qs(t!("se-ui-use-default")));
            use_default.set_tool_tip(&qs(t!("dlls-se-use-default-tip")));
            v.add_widget(&use_default);
            let summary = small_grey("");
            v.add_widget(&summary);

            let body = QWidget::new_0a();
            v.add_widget(&body);
            let bl = QVBoxLayout::new_1a(&body);
            bl.set_contents_margins_4a(0, 0, 0, 0);
            // Everyday areas first; the rest folds away behind "Show advanced features".
            let show_advanced = QCheckBox::from_q_string(&qs(t!("dlls-se-show-advanced")));
            show_advanced.set_tool_tip(&qs(t!("dlls-se-show-advanced-tip")));
            let advanced = QWidget::new_0a();
            let al = QVBoxLayout::new_0a();
            let mut rows = Vec::new();
            for area in Area::ALL {
                let g = QGroupBox::from_q_string(&qs(area.label()));
                if area == ADVANCED[0] {
                    bl.add_widget(&show_advanced);
                    bl.add_widget(&advanced);
                    advanced.set_layout(&al);
                    al.set_contents_margins_4a(0, 0, 0, 0);
                }
                if ADVANCED.contains(&area) {
                    al.add_widget(&g);
                } else {
                    bl.add_widget(&g);
                }
                let gl = QGridLayout::new_1a(&g);
                gl.set_column_stretch(1, 1);
                gl.set_vertical_spacing(2);
                let mut row = 0;
                for k in KEYS.iter().filter(|k| k.area == area) {
                    let tip = qs(t!("se-ui-key-tip", key = k.key, default = k.default));
                    let (ctrl, name): (Ctrl, Ptr<QWidget>) = match k.kind {
                        Kind::Bool => {
                            let c = QCheckBox::from_q_string(&qs(k.label()));
                            gl.add_widget_5a(&c, row, 0, 1, 2);
                            let p = c.as_ptr().static_upcast();
                            (Ctrl::Check(c), p)
                        }
                        kind => {
                            let caption = QLabel::from_q_string(&qs(k.label()));
                            caption.set_tool_tip(&tip);
                            let ctrl = match kind {
                                Kind::Int { min, max } => {
                                    let s = QSpinBox::new_0a();
                                    s.set_range(min as i32, max as i32);
                                    Ctrl::Spin(s)
                                }
                                Kind::Choice(opts) => {
                                    let c = QComboBox::new_0a();
                                    for v in opts {
                                        c.add_item_q_string_q_variant(&qs(k.choice_text(v)), &QVariant::from_q_string(&qs(*v)));
                                    }
                                    Ctrl::Combo(c)
                                }
                                _ => {
                                    let e = QLineEdit::new();
                                    e.set_placeholder_text(&qs(k.default));
                                    Ctrl::Line(e)
                                }
                            };
                            gl.add_widget_3a(&caption, row, 0);
                            gl.add_widget_3a(ctrl.widget(), row, 1);
                            let p = caption.as_ptr().static_upcast();
                            caption.into_ptr();
                            (ctrl, p)
                        }
                    };
                    ctrl.widget().set_tool_tip(&tip);
                    row += 1;
                    let help = if k.group == Group::Sim { format!("{} {}", k.help(), t!("dlls-se-must-match")) } else { k.help() };
                    let h = small_grey(&help);
                    h.set_contents_margins_4a(22, 0, 0, 4);
                    gl.add_widget_5a(&h, row, 0, 1, 2);
                    row += 1;
                    rows.push(Row { key: k.key, default: k.default, name, ctrl });
                }
            }
            let other = label("");
            colored(&other, &icons::amber());
            bl.add_widget(&other);
            let form = Rc::new(Form { rows, base: RefCell::new(SeConfig::default()), other });
            Features { root, target, use_default, reset, as_text, summary, body, show_advanced, advanced, form, loading: Rc::new(Cell::new(false)) }
        }
    }

    fn default_mode(&self) -> bool {
        unsafe { self.target.current_data_0a().to_int_0a() == TARGET_DEFAULT }
    }

    /// Fill the controls from the active profile (or the default).
    pub unsafe fn load(&self, app: &Rc<App>) {
        let default = dll::default_se_config(&app.ctx.settings());
        let (name, own) = {
            let st = app.st.borrow();
            let p = st.active();
            (p.name.clone(), p.se_config.clone())
        };
        self.target.set_item_text(0, &qs(t!("dlls-se-target-profile", profile = &name)));
        let default_mode = self.default_mode();
        self.use_default.set_visible(!default_mode);
        self.use_default.set_checked(own.is_none());
        let shown = if default_mode { default.clone() } else { own.clone().unwrap_or_else(|| default.clone()) };
        self.loading.set(true);
        self.form.load(&shown);
        self.loading.set(false);
        // A changed advanced value is never hidden.
        let advanced_changed = KEYS.iter().any(|k| ADVANCED.contains(&k.area) && !shown.is_default(k.key));
        if advanced_changed && !self.show_advanced.is_checked() {
            self.show_advanced.set_checked(true);
        }
        self.show_advanced.set_enabled(!advanced_changed);
        self.advanced.set_visible(self.show_advanced.is_checked());
        let editable = default_mode || own.is_some();
        self.body.set_enabled(editable);
        self.reset.set_enabled(editable);
        self.as_text.set_enabled(editable);
        let changed = crate::settings_ui::se_changed(&shown);
        self.summary.set_text(&qs(match (default_mode, own.is_some(), changed.is_empty()) {
            (false, false, _) => t!("dlls-se-summary-uses-default"),
            (_, _, true) => t!("dlls-se-summary-all-default"),
            _ => t!("dlls-se-summary-changed", keys = changed.join(", ")),
        }));
    }

    /// Store what the controls show, where the panel points.
    unsafe fn save(&self, app: &Rc<App>, c: SeConfig) {
        if self.default_mode() {
            let mut s = app.ctx.settings();
            s.se_config_default = Some(c);
            if let Err(e) = app.ctx.set_settings(s) {
                crate::dialogs::error(&app.window, &e);
            }
        } else {
            app.st.borrow_mut().update_active(|p| p.se_config = Some(c));
        }
        app.mark(DIRTY_LAUNCH);
    }

    pub unsafe fn connect(self: &Rc<Self>, app: &Rc<App>) {
        let w = &app.window;
        for r in &self.form.rows {
            let (f, a) = (self.clone(), app.clone());
            let changed = move || {
                if a.suppress.get() || f.loading.get() {
                    return;
                }
                f.form.refresh();
                f.save(&a, f.form.read());
            };
            match &r.ctrl {
                Ctrl::Check(c) => c.toggled().connect(&SlotOfBool::new(w, move |_| changed())),
                Ctrl::Spin(s) => s.value_changed().connect(&SlotOfInt::new(w, move |_| changed())),
                Ctrl::Combo(c) => c.activated().connect(&SlotOfInt::new(w, move |_| changed())),
                Ctrl::Line(e) => e.editing_finished().connect(&SlotNoArgs::new(w, move || changed())),
            };
        }
        let adv = self.advanced.as_ptr();
        self.show_advanced.toggled().connect(&SlotOfBool::new(w, move |on| adv.set_visible(on)));
        let (f, a) = (self.clone(), app.clone());
        self.target.activated().connect(&SlotOfInt::new(w, move |_| f.load(&a)));
        // Ticked: the profile follows the default. Unticked: it starts its own copy of it.
        let (f, a) = (self.clone(), app.clone());
        self.use_default.clicked().connect(&SlotOfBool::new(w, move |on| {
            let own = (!on).then(|| dll::default_se_config(&a.ctx.settings()));
            a.st.borrow_mut().update_active(|p| p.se_config = own);
            a.mark(DIRTY_LAUNCH);
            f.load(&a);
        }));
        let (f, a) = (self.clone(), app.clone());
        self.reset.clicked().connect(&SlotNoArgs::new(w, move || {
            if crate::dialogs::confirm(&a.window, &t!("se-ui-reset"), &t!("dlls-se-reset-question")) {
                // Unknown keys go too: "defaults" means the file as the DLL ships it.
                f.save(&a, SeConfig::default());
                f.load(&a);
            }
        }));
        let (f, a) = (self.clone(), app.clone());
        self.as_text.clicked().connect(&SlotNoArgs::new(w, move || {
            let text = f.form.read().render();
            if let Some(t) = crate::dialogs::edit_text(&a.window, "script_extender.cfg", &t!("se-ui-edit-intro"), &text) {
                f.save(&a, SeConfig::parse(&t));
                f.load(&a);
            }
        }));
    }
}
