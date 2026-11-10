//! The Conflicts and Logs tabs, and the crash-details dialog.

use crate::app::App;
use crate::details::{colored, label};
use crate::icons;
use qt_core::{qs, QBox, QListOfQString, QVariant, SlotNoArgs, SlotOfBool, SlotOfInt, SlotOfQString};
use qt_gui::QBrush;
use qt_widgets::{
    q_header_view::ResizeMode, QCheckBox, QComboBox, QHBoxLayout, QLabel, QLineEdit, QListWidget, QListWidgetItem, QPlainTextEdit, QPushButton, QSplitter,
    QStackedWidget, QTreeWidget, QTreeWidgetItem, QVBoxLayout, QWidget,
};
use std::cell::RefCell;
use std::rc::Rc;
use tkmm_core::conflicts::{summarize, ConflictReport, PairSummary};
use tkmm_core::fmt::format_date;
use tkmm_core::{history, logs};

pub struct ConflictsUi {
    pub root: QBox<QWidget>,
    /// 0 = mod pairs (the default), 1 = every file.
    views: QBox<QStackedWidget>,
    by_pair: QBox<QPushButton>,
    all_files: QBox<QPushButton>,
    pairs: QBox<QListWidget>,
    pair_title: QBox<QLabel>,
    pair_text: QBox<QLabel>,
    pair_files: QBox<QPushButton>,
    pair_show: QBox<QPushButton>,
    pub packs: QBox<QListWidget>,
    pub table: QBox<QTreeWidget>,
    pub filter: QBox<QLineEdit>,
    pub info: QBox<QLabel>,
    pub rescan: QBox<QPushButton>,
    report: RefCell<Option<ConflictReport>>,
    summary: RefCell<Vec<PairSummary>>,
    /// The file view shows only this pair's files (from "Show files").
    pair_filter: RefCell<Option<(String, String)>>,
}

impl ConflictsUi {
    pub fn build() -> ConflictsUi {
        unsafe {
            let root = QWidget::new_0a();
            let v = QVBoxLayout::new_1a(&root);
            v.set_contents_margins_4a(0, 0, 0, 0);
            let explain = label(&t!("tabs-conflicts-explain"));
            colored(&explain, &icons::grey());
            v.add_widget(&explain);
            let bar = QHBoxLayout::new_0a();
            let by_pair = QPushButton::from_q_string(&qs(t!("tabs-conflicts-view-pairs")));
            let all_files = QPushButton::from_q_string(&qs(t!("tabs-conflicts-view-files")));
            for b in [&by_pair, &all_files] {
                b.set_checkable(true);
                b.set_auto_exclusive(true);
                bar.add_widget(b);
            }
            by_pair.set_checked(true);
            let filter = QLineEdit::new();
            filter.set_placeholder_text(&qs(t!("tabs-conflicts-filter")));
            filter.set_clear_button_enabled(true);
            filter.set_visible(false);
            bar.add_widget_2a(&filter, 1);
            bar.add_stretch_1a(0);
            let info = QLabel::new();
            bar.add_widget(&info);
            let rescan = QPushButton::from_q_string(&qs(t!("tabs-conflicts-rescan")));
            bar.add_widget(&rescan);
            v.add_layout_1a(&bar);
            let views = QStackedWidget::new_0a();
            v.add_widget_2a(&views, 1);

            // Mod pairs: a card per pair on the left, what it means on the right.
            let psplit = QSplitter::new();
            let pairs = QListWidget::new_0a();
            pairs.set_style_sheet(&qs(crate::dll_tab::CARD_STYLE));
            pairs.set_word_wrap(true);
            psplit.add_widget(&pairs);
            let detail = QWidget::new_0a();
            let dl = QVBoxLayout::new_1a(&detail);
            dl.set_contents_margins_4a(10, 0, 4, 0);
            let pair_title = label("");
            crate::details::bold(&pair_title);
            dl.add_widget(&pair_title);
            let pair_text = label("");
            dl.add_widget(&pair_text);
            let br = QHBoxLayout::new_0a();
            dl.add_layout_1a(&br);
            let pair_files = QPushButton::from_q_string(&qs(t!("tabs-conflicts-pair-files")));
            let pair_show = QPushButton::from_q_string(&qs(t!("tabs-conflicts-pair-show")));
            pair_show.set_tool_tip(&qs(t!("tabs-conflicts-pair-show-tip")));
            br.add_widget(&pair_files);
            br.add_widget(&pair_show);
            br.add_stretch_1a(1);
            dl.add_stretch_1a(1);
            psplit.add_widget(&detail);
            psplit.set_stretch_factor(0, 3);
            psplit.set_stretch_factor(1, 2);
            views.add_widget(&psplit);

            let split = QSplitter::new();
            let packs = QListWidget::new_0a();
            let table = QTreeWidget::new_0a();
            let h = QListOfQString::new_0a();
            h.append_q_string(&qs(t!("tabs-conflicts-col-file")));
            h.append_q_string(&qs(t!("tabs-conflicts-col-packs")));
            table.set_header_labels(&h);
            table.set_root_is_decorated(false);
            table.set_uniform_row_heights(true);
            table.header().set_section_resize_mode_2a(0, ResizeMode::Stretch);
            table.header().set_section_resize_mode_2a(1, ResizeMode::Stretch);
            split.add_widget(&packs);
            split.add_widget(&table);
            split.set_stretch_factor(0, 1);
            split.set_stretch_factor(1, 3);
            views.add_widget(&split);
            ConflictsUi {
                root,
                views,
                by_pair,
                all_files,
                pairs,
                pair_title,
                pair_text,
                pair_files,
                pair_show,
                packs,
                table,
                filter,
                info,
                rescan,
                report: RefCell::new(None),
                summary: RefCell::new(Vec::new()),
                pair_filter: RefCell::new(None),
            }
        }
    }

    pub unsafe fn connect(&self, app: &Rc<App>) {
        let w = &app.window;
        let this = app.clone();
        self.rescan.clicked().connect(&SlotNoArgs::new(w, move || this.run_conflicts()));
        let this = app.clone();
        self.filter.text_changed().connect(&SlotOfQString::new(w, move |_| this.conflicts.fill_table(&this)));
        let this = app.clone();
        self.packs.current_row_changed().connect(&SlotOfInt::new(w, move |_| {
            // Picking a pack in the list ends "Show files" for one pair.
            if !this.suppress.get() {
                *this.conflicts.pair_filter.borrow_mut() = None;
            }
            this.conflicts.fill_table(&this);
        }));
        let this = app.clone();
        self.by_pair.clicked().connect(&SlotNoArgs::new(w, move || this.conflicts.show_view(&this, 0)));
        let this = app.clone();
        self.all_files.clicked().connect(&SlotNoArgs::new(w, move || {
            *this.conflicts.pair_filter.borrow_mut() = None;
            this.conflicts.show_view(&this, 1);
        }));
        let this = app.clone();
        self.pairs.current_row_changed().connect(&SlotOfInt::new(w, move |_| this.conflicts.fill_pair(&this)));
        let this = app.clone();
        self.pair_files.clicked().connect(&SlotNoArgs::new(w, move || {
            let Some(p) = this.conflicts.current_pair() else { return };
            *this.conflicts.pair_filter.borrow_mut() = Some((p.winner, p.loser));
            this.conflicts.show_view(&this, 1);
        }));
        let this = app.clone();
        self.pair_show.clicked().connect(&SlotNoArgs::new(w, move || {
            let Some(p) = this.conflicts.current_pair() else { return };
            {
                let mut st = this.st.borrow_mut();
                st.selected = vec![p.loser.clone(), p.winner.clone()];
                st.focused = Some(p.winner.clone());
            }
            this.ml.reveal_focused.set(true);
            this.set_page(crate::app::PAGE_MODS);
            this.mark(crate::app::DIRTY_LIST | crate::app::DIRTY_DETAILS);
        }));
    }

    unsafe fn show_view(&self, app: &Rc<App>, i: i32) {
        self.views.set_current_index(i);
        self.by_pair.set_checked(i == 0);
        self.all_files.set_checked(i == 1);
        self.filter.set_visible(i == 1);
        if i == 1 {
            app.suppress.set(true);
            self.packs.set_current_row_1a(0);
            app.suppress.set(false);
            self.fill_table(app);
        } else {
            self.info.set_text(&qs(t!("tabs-conflicts-pairs-count", count = self.summary.borrow().len())));
        }
    }

    unsafe fn current_pair(&self) -> Option<PairSummary> {
        let i = self.pairs.current_row();
        if i < 0 {
            return None;
        }
        self.summary.borrow().get(i as usize).cloned()
    }

    /// The right-hand text for the selected pair.
    unsafe fn fill_pair(&self, app: &Rc<App>) {
        let Some(p) = self.current_pair() else {
            self.pair_title.set_text(&qs(""));
            self.pair_text.set_text(&qs(if self.summary.borrow().is_empty() { t!("tabs-conflicts-none") } else { String::new() }));
            self.pair_files.set_visible(false);
            self.pair_show.set_visible(false);
            return;
        };
        let st = app.st.borrow();
        let (winner, loser) = (st.title_of(&p.winner), st.title_of(&p.loser));
        self.pair_title.set_text(&qs(t!("tabs-conflicts-pair-title", winner = &winner, loser = &loser, count = p.files)));
        let kinds: Vec<String> = p.kinds.iter().map(|(k, n)| t!("tabs-conflicts-kind-count", kind = k.label(), count = *n)).collect();
        let meaning = if p.check { t!("tabs-conflicts-pair-check", winner = &winner, loser = &loser) } else { t!("tabs-conflicts-pair-fine", winner = &winner, loser = &loser) };
        self.pair_text.set_text(&qs(format!("{}\n\n{meaning}\n\n{}", kinds.join("\n"), t!("tabs-conflicts-pair-order", winner = &winner, loser = &loser))));
        self.pair_files.set_visible(true);
        self.pair_show.set_visible(true);
    }

    pub unsafe fn set_busy(&self, busy: bool) {
        self.rescan.set_enabled(!busy);
        if busy {
            self.info.set_text(&qs(t!("tabs-conflicts-scanning")));
        }
    }

    pub unsafe fn show_report(&self, app: &Rc<App>, r: ConflictReport) {
        self.set_busy(false);
        self.packs.clear();
        let all = QListWidgetItem::from_q_string(&qs(t!("tabs-conflicts-all", count = r.conflicts.len())));
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
                let it = QListWidgetItem::from_q_string(&qs(t!("tabs-conflicts-unreadable", pack = st.title_of(k))));
                it.set_tool_tip(&qs(e));
                it.set_foreground(&QBrush::from_q_color(&icons::red()));
                self.packs.add_item_q_list_widget_item(it.into_ptr());
            }
        }
        let summary = summarize(&r);
        self.pairs.clear();
        {
            let st = app.st.borrow();
            for p in &summary {
                let main = p.kinds.first().map(|(k, _)| k.label()).unwrap_or_default();
                let text = format!(
                    "{}\n{}",
                    t!("tabs-conflicts-pair-card", winner = st.title_of(&p.winner), loser = st.title_of(&p.loser), count = p.files),
                    t!("tabs-conflicts-pair-card-sub", kind = main, level = if p.check { t!("tabs-conflicts-level-check") } else { t!("tabs-conflicts-level-fine") })
                );
                let it = QListWidgetItem::from_q_string(&qs(text));
                if p.check {
                    it.set_foreground(&QBrush::from_q_color(&icons::amber()));
                }
                self.pairs.add_item_q_list_widget_item(it.into_ptr());
            }
        }
        *self.summary.borrow_mut() = summary;
        *self.report.borrow_mut() = Some(r);
        self.packs.set_current_row_1a(0);
        self.fill_table(app);
        self.pairs.set_current_row_1a(0);
        self.fill_pair(app);
        let view = self.views.current_index();
        self.show_view(app, view);
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
        let pair = self.pair_filter.borrow().clone();
        let st = app.st.borrow();
        let mut shown = 0;
        for c in &r.conflicts {
            if (!only.is_empty() && !c.packs.contains(&only)) || (!q.is_empty() && !c.path.contains(&q)) {
                continue;
            }
            // One pair: files the winner wins and the loser also has.
            if let Some((w, l)) = &pair {
                if c.packs.last() != Some(w) || !c.packs.contains(l) {
                    continue;
                }
            }
            shown += 1;
            if shown > 5000 {
                break;
            }
            let it = QTreeWidgetItem::new();
            it.set_text(0, &qs(&c.path));
            let names: Vec<String> = c.packs.iter().map(|k| st.title_of(k)).collect();
            it.set_text(1, &qs(names.join("  →  ")));
            it.set_tool_tip(1, &qs(t!("tabs-conflicts-winner", pack = names.last().cloned().unwrap_or_default())));
            self.table.add_top_level_item(it.into_ptr());
        }
        self.info.set_text(&qs(match &pair {
            Some((w, l)) => t!("tabs-conflicts-shown-pair", count = shown, winner = st.title_of(w), loser = st.title_of(l)),
            None => t!("tabs-conflicts-shown", count = shown),
        }));
    }
}

/// The Troubleshooting tab: what happened the last time the game ran, in words, with Send
/// report; the raw log files fold away underneath for modders.
pub struct LogsUi {
    pub root: QBox<QWidget>,
    summary_layout: QBox<QVBoxLayout>,
    summary: RefCell<Option<QBox<QWidget>>>,
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
            let outer = QVBoxLayout::new_1a(&root);
            outer.set_contents_margins_4a(0, 0, 0, 0);
            let g = qt_widgets::QGroupBox::from_q_string(&qs(t!("tabs-trouble-last-game")));
            let summary_layout = QVBoxLayout::new_1a(&g);
            outer.add_widget(&g);
            let show_logs = QPushButton::from_q_string(&qs(t!("tabs-trouble-show-logs")));
            show_logs.set_checkable(true);
            show_logs.set_tool_tip(&qs(t!("tabs-trouble-show-logs-tip")));
            let sl = QHBoxLayout::new_0a();
            outer.add_layout_1a(&sl);
            sl.add_widget(&show_logs);
            sl.add_stretch_1a(1);
            let logs_box = QWidget::new_0a();
            outer.add_widget_2a(&logs_box, 1);
            logs_box.set_visible(false);
            let spacer = QWidget::new_0a();
            outer.add_widget_2a(&spacer, 1);
            let sp = spacer.as_ptr();
            let lb = logs_box.as_ptr();
            show_logs.toggled().connect(&SlotOfBool::new(&root, move |on| {
                lb.set_visible(on);
                sp.set_visible(!on);
            }));
            spacer.into_ptr();
            let v = QVBoxLayout::new_1a(&logs_box);
            v.set_contents_margins_4a(0, 0, 0, 0);
            let bar = QHBoxLayout::new_0a();
            let source = QComboBox::new_0a();
            source.set_minimum_width(240);
            bar.add_widget(&source);
            let filter = QLineEdit::new();
            filter.set_placeholder_text(&qs(t!("tabs-logs-filter")));
            filter.set_clear_button_enabled(true);
            bar.add_widget_2a(&filter, 1);
            let follow = QCheckBox::from_q_string(&qs(t!("tabs-logs-follow")));
            follow.set_checked(true);
            bar.add_widget(&follow);
            let show_file = QPushButton::from_q_string(&qs(t!("tabs-logs-show-file")));
            bar.add_widget(&show_file);
            v.add_layout_1a(&bar);
            let text = QPlainTextEdit::new();
            text.set_read_only(true);
            text.set_line_wrap_mode(qt_widgets::q_plain_text_edit::LineWrapMode::NoWrap);
            let f = qt_gui::QFont::new_copy(text.font());
            f.set_family(&qs("Consolas"));
            text.set_font(&f);
            text.set_placeholder_text(&qs(t!("tabs-logs-placeholder")));
            v.add_widget_2a(&text, 1);
            LogsUi { root, summary_layout, summary: RefCell::new(None), source, filter, follow, text, show_file, last: RefCell::new(String::new()) }
        }
    }

    /// Rebuild the "Last game" box from the launch history.
    pub unsafe fn refresh_summary(&self, app: &Rc<App>) {
        if let Some(old) = self.summary.borrow_mut().take() {
            old.delete_later();
        }
        let w = QWidget::new_0a();
        let l = QVBoxLayout::new_1a(&w);
        l.set_contents_margins_4a(0, 0, 0, 0);
        let (running, se_on, se_version) = {
            let st = app.st.borrow();
            (st.game_running, st.active().dll, st.dll_version())
        };
        let report = history::crash_report(None);
        let mut crashed = false;
        let head = match &report {
            None => (t!("tabs-trouble-never"), icons::grey()),
            Some(_) if running => (t!("tabs-trouble-running"), icons::green()),
            Some(r) => {
                let when = format_date(r.record.started);
                match (r.record.ended, r.record.exit_code) {
                    (_, Some(0)) => (t!("tabs-trouble-closed", date = when, profile = &r.record.profile), icons::green()),
                    (None, _) => (t!("tabs-trouble-unknown", date = when, profile = &r.record.profile), icons::amber()),
                    _ => {
                        crashed = true;
                        (t!("tabs-trouble-crashed", date = when, profile = &r.record.profile), icons::red())
                    }
                }
            }
        };
        let h = label(&head.0);
        crate::details::bold(&h);
        colored(&h, &head.1);
        l.add_widget(&h);
        l.add_widget(&label(&match (se_on, &se_version) {
            (true, Some(v)) => t!("tabs-trouble-se-on", version = v),
            (true, None) => t!("tabs-trouble-se-missing"),
            (false, _) => t!("tabs-trouble-se-off"),
        }));
        if crashed {
            if let Some(d) = report.as_ref().and_then(|r| r.diff.as_ref()) {
                let changes = d.added.len() + d.changed.len() + d.removed.len();
                l.add_widget(&label(&if changes == 0 && !d.reordered {
                    t!("tabs-trouble-nothing-changed")
                } else {
                    t!("tabs-trouble-changed", count = changes)
                }));
            }
            let what = label(&t!("tabs-trouble-what-to-do"));
            colored(&what, &icons::grey());
            l.add_widget(&what);
        }
        let row = QHBoxLayout::new_0a();
        l.add_layout_1a(&row);
        let send = QPushButton::from_q_string(&qs(t!("launch-panel-send-report")));
        send.set_tool_tip(&qs(t!("launch-panel-send-report-tip")));
        let this = app.clone();
        send.clicked().connect(&SlotNoArgs::new(&send, move || this.send_report()));
        row.add_widget(&send);
        if report.is_some() {
            let more = QPushButton::from_q_string(&qs(t!("tabs-trouble-changes")));
            more.set_tool_tip(&qs(t!("tabs-trouble-changes-tip")));
            let this = app.clone();
            more.clicked().connect(&SlotNoArgs::new(&more, move || crash_dialog(&this, None)));
            row.add_widget(&more);
        }
        row.add_stretch_1a(1);
        self.summary_layout.add_widget(&w);
        *self.summary.borrow_mut() = Some(w);
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
        // The source id ("script_log", ...) sits next to the path: the newest log's path
        // changes when the game writes a new one, and the label is translated text.
        let id_role = qt_core::ItemDataRole::UserRole.to_int() + 1;
        let current_id = {
            let i = self.source.current_index();
            if i >= 0 { self.source.item_data_2a(i, id_role).to_string().to_std_string() } else { String::new() }
        };
        app.suppress.set(true);
        self.source.clear();
        for s in &sources {
            self.source.add_item_q_string_q_variant(&qs(&s.label), &QVariant::from_q_string(&qs(&s.path)));
            self.source.set_item_data_3a(self.source.count() - 1, &QVariant::from_q_string(&qs(&s.id)), id_role);
        }
        let mut idx = sources.iter().position(|s| s.path == current).or_else(|| sources.iter().position(|s| s.id == current_id)).unwrap_or(0) as i32;
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
    crate::dialogs::custom(&app.window, &t!("tabs-crash-title"), 760, 560, |l, _| {
        l.add_widget(&label(&t!("tabs-crash-intro")));
        match &report {
            None => l.add_widget(&label(&t!("tabs-crash-no-record"))),
            Some(r) => {
                let head = label(&match &r.baseline {
                    Some(b) => t!("tabs-crash-changed-since", date = format_date(b.started), profile = &b.profile),
                    None => t!("tabs-crash-no-baseline"),
                });
                crate::details::bold(&head);
                l.add_widget(&head);
                if let Some(d) = &r.diff {
                    if d.added.is_empty() && d.removed.is_empty() && d.changed.is_empty() && !d.reordered {
                        l.add_widget(&label(&t!("tabs-crash-nothing-changed")));
                    }
                    for (items, what, c) in [(&d.added, "tabs-crash-added", icons::green()), (&d.changed, "tabs-crash-file-changed", icons::amber()), (&d.removed, "tabs-crash-removed", icons::red())] {
                        for f in items.iter() {
                            let w = label(&match what {
                                "tabs-crash-added" => t!("tabs-crash-added", pack = f),
                                "tabs-crash-file-changed" => t!("tabs-crash-file-changed", pack = f),
                                _ => t!("tabs-crash-removed", pack = f),
                            });
                            colored(&w, &c);
                            l.add_widget(&w);
                        }
                    }
                    if d.reordered {
                        l.add_widget(&label(&t!("tabs-crash-reordered")));
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
