//! The Conflicts and Logs tabs, and the crash-details dialog.

use crate::app::App;
use crate::details::{colored, label};
use crate::icons;
use qt_core::{qs, QBox, QListOfQString, QVariant, SlotNoArgs, SlotOfInt, SlotOfQString};
use qt_gui::QBrush;
use qt_widgets::{
    q_header_view::ResizeMode, QCheckBox, QComboBox, QHBoxLayout, QLabel, QLineEdit, QListWidget, QListWidgetItem, QPlainTextEdit, QPushButton, QSplitter,
    QTreeWidget, QTreeWidgetItem, QVBoxLayout, QWidget,
};
use std::cell::RefCell;
use std::rc::Rc;
use tkmm_core::conflicts::ConflictReport;
use tkmm_core::fmt::format_date;
use tkmm_core::{history, logs};

pub struct ConflictsUi {
    pub root: QBox<QWidget>,
    pub packs: QBox<QListWidget>,
    pub table: QBox<QTreeWidget>,
    pub filter: QBox<QLineEdit>,
    pub info: QBox<QLabel>,
    pub rescan: QBox<QPushButton>,
    report: RefCell<Option<ConflictReport>>,
}

impl ConflictsUi {
    pub fn build() -> ConflictsUi {
        unsafe {
            let root = QWidget::new_0a();
            let v = QVBoxLayout::new_1a(&root);
            v.set_contents_margins_4a(0, 0, 0, 0);
            let bar = QHBoxLayout::new_0a();
            let filter = QLineEdit::new();
            filter.set_placeholder_text(&qs("Filter paths (e.g. db/land_units)"));
            filter.set_clear_button_enabled(true);
            bar.add_widget_2a(&filter, 1);
            let info = QLabel::new();
            bar.add_widget(&info);
            let rescan = QPushButton::from_q_string(&qs("Rescan"));
            bar.add_widget(&rescan);
            v.add_layout_1a(&bar);
            let split = QSplitter::new();
            let packs = QListWidget::new_0a();
            let table = QTreeWidget::new_0a();
            let h = QListOfQString::new_0a();
            h.append_q_string(&qs("File"));
            h.append_q_string(&qs("Provided by, in load order (last one wins)"));
            table.set_header_labels(&h);
            table.set_root_is_decorated(false);
            table.set_uniform_row_heights(true);
            table.header().set_section_resize_mode_2a(0, ResizeMode::Stretch);
            table.header().set_section_resize_mode_2a(1, ResizeMode::Stretch);
            split.add_widget(&packs);
            split.add_widget(&table);
            split.set_stretch_factor(0, 1);
            split.set_stretch_factor(1, 3);
            v.add_widget_2a(&split, 1);
            ConflictsUi { root, packs, table, filter, info, rescan, report: RefCell::new(None) }
        }
    }

    pub unsafe fn connect(&self, app: &Rc<App>) {
        let this = app.clone();
        self.rescan.clicked().connect(&SlotNoArgs::new(&app.window, move || this.run_conflicts()));
        let this = app.clone();
        self.filter.text_changed().connect(&SlotOfQString::new(&app.window, move |_| this.conflicts.fill_table(&this)));
        let this = app.clone();
        self.packs.current_row_changed().connect(&SlotOfInt::new(&app.window, move |_| this.conflicts.fill_table(&this)));
    }

    pub unsafe fn set_busy(&self, busy: bool) {
        self.rescan.set_enabled(!busy);
        if busy {
            self.info.set_text(&qs("Scanning…"));
        }
    }

    pub unsafe fn show_report(&self, app: &Rc<App>, r: ConflictReport) {
        self.set_busy(false);
        self.packs.clear();
        let all = QListWidgetItem::from_q_string(&qs(format!("All ({} shared files)", r.conflicts.len())));
        all.set_data(qt_core::ItemDataRole::UserRole.to_int(), &QVariant::from_q_string(&qs("")));
        self.packs.add_item_q_list_widget_item(all.into_ptr());
        let mut per: Vec<(&String, &usize)> = r.per_pack.iter().collect();
        per.sort_by(|a, b| b.1.cmp(a.1));
        {
            let st = app.st.borrow();
            for (k, n) in per {
                let it = QListWidgetItem::from_q_string(&qs(format!("{}  ({n})", st.title_of(k))));
                it.set_data(qt_core::ItemDataRole::UserRole.to_int(), &QVariant::from_q_string(&qs(k)));
                it.set_tool_tip(&qs(k));
                self.packs.add_item_q_list_widget_item(it.into_ptr());
            }
            for (k, e) in &r.errors {
                let it = QListWidgetItem::from_q_string(&qs(format!("⚠ {}: unreadable", st.title_of(k))));
                it.set_tool_tip(&qs(e));
                it.set_foreground(&QBrush::from_q_color(&icons::red()));
                self.packs.add_item_q_list_widget_item(it.into_ptr());
            }
        }
        *self.report.borrow_mut() = Some(r);
        self.packs.set_current_row_1a(0);
        self.fill_table(app);
    }

    unsafe fn fill_table(&self, app: &Rc<App>) {
        self.table.clear();
        let report = self.report.borrow();
        let Some(r) = report.as_ref() else { return };
        let only = {
            let cur = self.packs.current_item();
            if cur.is_null() { String::new() } else { cur.data(qt_core::ItemDataRole::UserRole.to_int()).to_string().to_std_string() }
        };
        let q = self.filter.text().to_std_string().to_lowercase();
        let st = app.st.borrow();
        let mut shown = 0;
        for c in &r.conflicts {
            if (!only.is_empty() && !c.packs.contains(&only)) || (!q.is_empty() && !c.path.contains(&q)) {
                continue;
            }
            shown += 1;
            if shown > 5000 {
                break;
            }
            let it = QTreeWidgetItem::new();
            it.set_text(0, &qs(&c.path));
            let names: Vec<String> = c.packs.iter().map(|k| st.title_of(k)).collect();
            it.set_text(1, &qs(names.join("  →  ")));
            it.set_tool_tip(1, &qs(format!("Winner: {}", names.last().cloned().unwrap_or_default())));
            self.table.add_top_level_item(it.into_ptr());
        }
        self.info.set_text(&qs(format!("{shown} shared files")));
    }
}

pub struct LogsUi {
    pub root: QBox<QWidget>,
    pub source: QBox<QComboBox>,
    pub filter: QBox<QLineEdit>,
    pub follow: QBox<QCheckBox>,
    pub text: QBox<QPlainTextEdit>,
    pub show_file: QBox<QPushButton>,
    last: RefCell<String>,
}

impl LogsUi {
    pub fn build() -> LogsUi {
        unsafe {
            let root = QWidget::new_0a();
            let v = QVBoxLayout::new_1a(&root);
            v.set_contents_margins_4a(0, 0, 0, 0);
            let bar = QHBoxLayout::new_0a();
            let source = QComboBox::new_0a();
            source.set_minimum_width(240);
            bar.add_widget(&source);
            let filter = QLineEdit::new();
            filter.set_placeholder_text(&qs("Filter lines (e.g. error)"));
            filter.set_clear_button_enabled(true);
            bar.add_widget_2a(&filter, 1);
            let follow = QCheckBox::from_q_string(&qs("Follow"));
            follow.set_checked(true);
            bar.add_widget(&follow);
            let show_file = QPushButton::from_q_string(&qs("Show file"));
            bar.add_widget(&show_file);
            v.add_layout_1a(&bar);
            let text = QPlainTextEdit::new();
            text.set_read_only(true);
            text.set_line_wrap_mode(qt_widgets::q_plain_text_edit::LineWrapMode::NoWrap);
            let f = qt_gui::QFont::new_copy(text.font());
            f.set_family(&qs("Consolas"));
            text.set_font(&f);
            text.set_placeholder_text(&qs("Logs appear here once the game has run: the script extender log, lua_mod_log.txt, and the newest script / ironic logs."));
            v.add_widget_2a(&text, 1);
            LogsUi { root, source, filter, follow, text, show_file, last: RefCell::new(String::new()) }
        }
    }

    pub unsafe fn connect(&self, app: &Rc<App>) {
        let this = app.clone();
        self.source.activated().connect(&SlotOfInt::new(&app.window, move |_| {
            this.logs.last.borrow_mut().clear();
            this.logs.poll(&this);
        }));
        let this = app.clone();
        self.filter.text_changed().connect(&SlotOfQString::new(&app.window, move |_| {
            this.logs.last.borrow_mut().clear();
            this.logs.poll(&this);
        }));
        let this = app.clone();
        self.show_file.clicked().connect(&SlotNoArgs::new(&app.window, move || {
            let p = this.logs.source.current_data_0a().to_string().to_std_string();
            if !p.is_empty() {
                crate::details::reveal(&p);
            }
        }));
    }

    /// Refresh the source list and the text (every 2 s while the tab is visible).
    pub unsafe fn poll(&self, app: &Rc<App>) {
        let sources = logs::log_sources(&app.ctx);
        let current = self.source.current_data_0a().to_string().to_std_string();
        let current_id = {
            let i = self.source.current_index();
            if i >= 0 { self.source.item_text(i).to_std_string() } else { String::new() }
        };
        app.suppress.set(true);
        self.source.clear();
        for s in &sources {
            self.source.add_item_q_string_q_variant(&qs(&s.label), &QVariant::from_q_string(&qs(&s.path)));
        }
        let mut idx = sources.iter().position(|s| s.path == current).or_else(|| sources.iter().position(|s| s.label == current_id)).unwrap_or(0) as i32;
        if sources.is_empty() {
            idx = -1;
        }
        self.source.set_current_index(idx);
        app.suppress.set(false);
        let Some(src) = sources.get(idx.max(0) as usize).filter(|_| idx >= 0) else {
            self.text.set_plain_text(&qs(""));
            return;
        };
        let text = logs::log_tail(&app.ctx, &src.path, Some(512 * 1024)).unwrap_or_else(|e| e);
        let q = self.filter.text().to_std_string().to_lowercase();
        let shown = if q.is_empty() { text } else { text.lines().filter(|l| l.to_lowercase().contains(&q)).collect::<Vec<_>>().join("\n") };
        if *self.last.borrow() == shown {
            return;
        }
        *self.last.borrow_mut() = shown.clone();
        let bar = self.text.vertical_scroll_bar();
        let keep = bar.value();
        self.text.set_plain_text(&qs(&shown));
        if self.follow.is_checked() {
            bar.set_value(bar.maximum());
        } else {
            bar.set_value(keep);
        }
    }
}

/// What changed since the last clean launch, plus the last log lines.
pub unsafe fn crash_dialog(app: &Rc<App>, id: Option<u64>) {
    let report = history::crash_report(id);
    let tail = {
        let sources = logs::log_sources(&app.ctx);
        let src = sources.iter().find(|s| s.id == "script_log").or_else(|| sources.iter().find(|s| s.id == "dll")).or(sources.first()).cloned();
        src.map(|s| {
            let t = logs::log_tail(&app.ctx, &s.path, Some(16 * 1024)).unwrap_or_default();
            let lines: Vec<&str> = t.lines().collect();
            format!("{}\n{}", s.label, lines[lines.len().saturating_sub(40)..].join("\n"))
        })
    };
    crate::dialogs::custom(&app.window, "Abnormal exit", 760, 560, |l, _| {
        l.add_widget(&label("An exit code other than 0 usually means a crash, but it is also what you get when the game is closed from Task Manager."));
        match &report {
            None => l.add_widget(&label("No launch record found.")),
            Some(r) => {
                let head = label(&match &r.baseline {
                    Some(b) => format!("Changed since the last clean launch ({}, {})", format_date(b.started), b.profile),
                    None => "No earlier launch that exited cleanly is on record yet.".to_string(),
                });
                crate::details::bold(&head);
                l.add_widget(&head);
                if let Some(d) = &r.diff {
                    if d.added.is_empty() && d.removed.is_empty() && d.changed.is_empty() && !d.reordered {
                        l.add_widget(&label("Nothing: same packs, same files, same order."));
                    }
                    for (items, what, c) in [(&d.added, "added", icons::green()), (&d.changed, "file changed", icons::amber()), (&d.removed, "removed", icons::red())] {
                        for f in items.iter() {
                            let w = label(&format!("{what}: {f}"));
                            colored(&w, &c);
                            l.add_widget(&w);
                        }
                    }
                    if d.reordered {
                        l.add_widget(&label("order: the load order of the common packs changed"));
                    }
                }
            }
        }
        if let Some(t) = &tail {
            let e = QPlainTextEdit::new();
            e.set_read_only(true);
            e.set_plain_text(&qs(t));
            let f = qt_gui::QFont::new_copy(e.font());
            f.set_family(&qs("Consolas"));
            e.set_font(&f);
            l.add_widget_2a(&e, 1);
        }
    });
}
