//! The mod list: a QTreeView over a QStandardItemModel rebuilt from `State`.
//!
//! In load order (no narrowing filters) separators are top-level rows with their packs as
//! children: native folding, a check box that toggles the group, and drag & drop of packs and
//! whole groups (InternalMove). A drop is applied to the profile by key (see `flush_drop`).
//! Sorting by another column or filtering shows a flat list without drag & drop.

use crate::app::{App, DIRTY_DETAILS, DIRTY_HEADER, DIRTY_LAUNCH, DIRTY_LIST};
use crate::icons;
use crate::state::SortKey;
use cpp_core::{CppBox, Ptr};
use qt_core::{
    qs, CheckState, ContextMenuPolicy, ItemDataRole, ItemFlag, QBox, QFlags, QItemSelection, QModelIndex, QPersistentModelIndex, QPoint, QPtr, QString, QVariant, SlotNoArgs,
    SlotOfBool, SlotOfInt, SlotOfQItemSelectionQItemSelection, SlotOfQModelIndex, SlotOfQModelIndexIntInt, SlotOfQString, SortOrder,
};
use qt_gui::{QBrush, QFont, QKeySequence, QListOfQStandardItem, QShortcut, QStandardItem, QStandardItemModel, SlotOfQStandardItem};
use qt_widgets::{
    q_abstract_item_view::{DragDropMode, SelectionMode},
    q_header_view::ResizeMode,
    QCheckBox, QComboBox, QHBoxLayout, QLabel, QLineEdit, QMenu, QPushButton, QTreeView, QVBoxLayout, QWidget, SlotOfQPoint,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use tkmm_core::fmt::{compare_pack_names, format_bytes, format_date};
use tkmm_core::groups::{self, GroupState};
use tkmm_core::packs::{ModSource, PackType};
use tkmm_core::profiles::ProfileEntry;
use tkmm_core::status::{self, StatusKind};
use tkmm_core::{profile_ops, update};

const COL_CHECK: i32 = 0;
const COL_STATUS: i32 = 1;
const COL_ORDER: i32 = 2;
const COL_TITLE: i32 = 3;
const COL_FLAGS: i32 = 4;
const COL_SOURCE: i32 = 5;
const COL_TYPE: i32 = 6;
const COL_SIZE: i32 = 7;
const COL_UPDATED: i32 = 8;
const COLUMN_COUNT: i32 = COL_UPDATED + 1;
const HEADERS: [&str; 9] = ["", "", "#", "Mod", "Notes", "Source", "Type", "Size", "Updated"];

const MOVIE_HEADER_KEY: &str = "__movies__";

/// Row-size presets: (settings value, label, row min-height px, font +pt, check box px,
/// check / status column widths).
pub const DENSITIES: [(&str, &str, i32, f64, i32, i32, i32); 3] = [
    ("compact", "Compact rows", 22, 0.0, 14, 44, 48),
    ("normal", "Normal rows", 30, 0.5, 18, 52, 56),
    ("large", "Large rows", 40, 2.0, 22, 60, 64),
];

/// Save and apply a row-size preset (from the list toolbar or Settings).
pub unsafe fn set_density(app: &Rc<App>, value: &str) {
    let mut s = app.ctx.settings();
    if s.list_density != value {
        s.list_density = value.to_string();
        if let Err(e) = app.ctx.set_settings(s) {
            crate::dialogs::error(&app.window, &e);
        }
    }
    app.ml.apply_density(value);
    app.mark(crate::app::DIRTY_SETTINGS);
}

/// The row-size choices, for the list toolbar and the Settings page.
pub unsafe fn density_combo() -> QBox<QComboBox> {
    let items: Vec<(&str, &str)> = DENSITIES.iter().map(|d| (d.1, d.0)).collect();
    combo(&items)
}

fn role_key() -> i32 {
    ItemDataRole::UserRole.to_int() + 1
}

pub struct ModListUi {
    pub root: QBox<QWidget>,
    pub search: QBox<QLineEdit>,
    pub source: QBox<QComboBox>,
    pub pack_type: QBox<QComboBox>,
    pub enabled: QBox<QComboBox>,
    pub status: QBox<QComboBox>,
    pub tag: QBox<QComboBox>,
    pub hidden: QBox<QCheckBox>,
    /// "Show missing (N)": reveals entries whose pack is not installed. Hidden when N is 0.
    pub missing: QBox<QCheckBox>,
    pub count: QBox<QLabel>,
    pub updated_btn: QBox<QPushButton>,
    pub group_btn: QBox<QPushButton>,
    pub sort_btn: QBox<QPushButton>,
    pub top_btn: QBox<QPushButton>,
    pub rescan_btn: QBox<QPushButton>,
    /// "Remove missing…": drops the uninstalled entries from the profile. Hidden when none.
    pub purge_btn: QBox<QPushButton>,
    pub density: QBox<QComboBox>,
    /// The view's font size before any row-size adjustment.
    base_point_size: f64,
    pub view: QBox<QTreeView>,
    pub model: QBox<QStandardItemModel>,
    pub pending_drop: RefCell<Vec<CppBox<QPersistentModelIndex>>>,
}

unsafe fn combo(items: &[(&str, &str)]) -> QBox<QComboBox> {
    let c = QComboBox::new_0a();
    for (label, value) in items {
        c.add_item_q_string_q_variant(&qs(*label), &QVariant::from_q_string(&qs(*value)));
    }
    c
}

unsafe fn item(text: &str) -> CppBox<QStandardItem> {
    let i = QStandardItem::new();
    i.set_text(&qs(text));
    i.set_editable(false);
    i
}

unsafe fn row_list(items: Vec<CppBox<QStandardItem>>) -> CppBox<QListOfQStandardItem> {
    let list = QListOfQStandardItem::new_0a();
    for it in items {
        list.append_q_standard_item(&it.into_ptr().as_mut_raw_ptr());
    }
    list
}

unsafe fn key_of(model: &QStandardItemModel, idx: &QModelIndex) -> String {
    let first = idx.sibling_at_column(COL_CHECK);
    model.data_2a(&first, role_key()).to_string().to_std_string()
}

impl ModListUi {
    pub fn build() -> ModListUi {
        unsafe {
            let root = QWidget::new_0a();
            let layout = QVBoxLayout::new_1a(&root);
            layout.set_contents_margins_4a(0, 0, 0, 0);
            layout.set_spacing(4);

            let bar = QHBoxLayout::new_0a();
            let search = QLineEdit::new();
            search.set_placeholder_text(&qs("Search title / file"));
            search.set_clear_button_enabled(true);
            search.set_minimum_width(200);
            bar.add_widget(&search);
            let source = combo(&[("All sources", "all"), ("Workshop", "workshop"), ("data/", "data"), ("Folders", "folder"), ("Nexus", "nexus")]);
            let pack_type = combo(&[("Mods + movies", "all"), ("Mod packs", "mod"), ("Movie packs", "movie")]);
            let enabled = combo(&[("Enabled + disabled", "all"), ("Enabled", "enabled"), ("Disabled", "disabled"), ("Updated since last played", "updated")]);
            let status = combo(&[("Any status", "all"), ("Update pending", "pending"), ("Older than game patch", "old"), ("Needs script extender", "se")]);
            let tag = combo(&[("Any tag", "")]);
            for c in [&source, &pack_type, &enabled, &status, &tag] {
                bar.add_widget(c);
            }
            let hidden = QCheckBox::from_q_string(&qs("Show hidden"));
            bar.add_widget(&hidden);
            let missing = QCheckBox::from_q_string(&qs("Show missing"));
            missing.set_tool_tip(&qs(
                "Show entries whose pack is not installed any more (unsubscribed or deleted). \
                 They are kept in the profile so the load order comes back if you re-subscribe.",
            ));
            missing.set_visible(false);
            bar.add_widget(&missing);
            bar.add_stretch_1a(1);
            layout.add_layout_1a(&bar);

            let bar2 = QHBoxLayout::new_0a();
            let count = QLabel::new();
            bar2.add_widget(&count);
            let updated_btn = QPushButton::new();
            updated_btn.set_flat(true);
            updated_btn.set_visible(false);
            bar2.add_widget(&updated_btn);
            bar2.add_stretch_1a(1);
            let group_btn = QPushButton::from_q_string(&qs("Add group"));
            group_btn.set_tool_tip(&qs("Add a separator; the mods below it form a group you can fold, toggle and drag"));
            let sort_btn = QPushButton::from_q_string(&qs("Sort A–Z"));
            sort_btn.set_tool_tip(&qs("Sort by pack file name (the CA launcher's default order), inside each group"));
            let top_btn = QPushButton::from_q_string(&qs("Enabled to top"));
            top_btn.set_tool_tip(&qs("Move enabled mods above disabled ones, inside each group"));
            let rescan_btn = QPushButton::from_q_string(&qs("Rescan"));
            rescan_btn.set_tool_tip(&qs("Look for added and removed packs now (also runs when the window regains focus)"));
            let purge_btn = QPushButton::new();
            purge_btn.set_visible(false);
            let density = density_combo();
            density.set_tool_tip(&qs("Height of the rows in the mod list (also in Settings)"));
            bar2.add_widget(&purge_btn);
            bar2.add_widget(&density);
            bar2.add_widget(&group_btn);
            bar2.add_widget(&sort_btn);
            bar2.add_widget(&top_btn);
            bar2.add_widget(&rescan_btn);
            layout.add_layout_1a(&bar2);

            let model = QStandardItemModel::new_0a();
            let labels = qt_core::QListOfQString::new_0a();
            for h in HEADERS {
                labels.append_q_string(&qs(h));
            }
            model.set_horizontal_header_labels(&labels);

            let view = QTreeView::new_0a();
            view.set_model(&model);
            view.set_alternating_row_colors(true);
            view.set_selection_mode(SelectionMode::ExtendedSelection);
            view.set_context_menu_policy(ContextMenuPolicy::CustomContextMenu);
            view.set_all_columns_show_focus(true);
            view.set_indentation(14);
            let header = view.header();
            header.set_sections_clickable(true);
            header.set_stretch_last_section(false);
            header.set_section_resize_mode_2a(COL_CHECK, ResizeMode::Fixed);
            header.resize_section(COL_CHECK, 44);
            header.set_section_resize_mode_2a(COL_STATUS, ResizeMode::Fixed);
            header.resize_section(COL_STATUS, 48);
            header.set_section_resize_mode_2a(COL_ORDER, ResizeMode::ResizeToContents);
            header.set_section_resize_mode_2a(COL_TITLE, ResizeMode::Stretch);
            for c in [COL_FLAGS, COL_SOURCE, COL_TYPE, COL_SIZE, COL_UPDATED] {
                header.set_section_resize_mode_2a(c, ResizeMode::ResizeToContents);
            }
            layout.add_widget_2a(&view, 1);
            let base_point_size = view.font().point_size_f();

            ModListUi {
                root,
                search,
                source,
                pack_type,
                enabled,
                status,
                tag,
                hidden,
                missing,
                count,
                updated_btn,
                group_btn,
                sort_btn,
                top_btn,
                rescan_btn,
                purge_btn,
                density,
                base_point_size,
                view,
                model,
                pending_drop: RefCell::new(Vec::new()),
            }
        }
    }

    pub unsafe fn connect(&self, app: &Rc<App>) {
        let w = &app.window;
        let this = app.clone();
        self.search.text_changed().connect(&SlotOfQString::new(w, move |t| {
            this.st.borrow_mut().filters.search = t.to_std_string();
            this.mark(DIRTY_LIST);
        }));
        for (c, which) in [(&self.source, 0), (&self.pack_type, 1), (&self.enabled, 2), (&self.status, 3), (&self.tag, 4)] {
            let this = app.clone();
            let cp: QPtr<QComboBox> = QPtr::new(c.as_ptr());
            c.activated().connect(&SlotOfInt::new(w, move |_| {
                let v = cp.current_data_0a().to_string().to_std_string();
                {
                    let mut st = this.st.borrow_mut();
                    match which {
                        0 => st.filters.source = v,
                        1 => st.filters.pack_type = v,
                        2 => st.filters.enabled = v,
                        3 => st.filters.status = v,
                        _ => st.filters.tag = Some(v).filter(|s| !s.is_empty()),
                    }
                }
                this.mark(DIRTY_LIST);
            }));
        }
        let this = app.clone();
        self.hidden.clicked().connect(&SlotOfBool::new(w, move |on| {
            this.st.borrow_mut().filters.show_hidden = on;
            this.mark(DIRTY_LIST);
        }));
        let this = app.clone();
        self.missing.clicked().connect(&SlotOfBool::new(w, move |on| {
            this.st.borrow_mut().filters.show_missing = on;
            this.mark(DIRTY_LIST);
        }));
        let this = app.clone();
        self.rescan_btn.clicked().connect(&SlotNoArgs::new(w, move || this.rescan()));
        let this = app.clone();
        self.purge_btn.clicked().connect(&SlotNoArgs::new(w, move || this.purge_missing()));
        let this = app.clone();
        self.updated_btn.clicked().connect(&SlotNoArgs::new(w, move || {
            this.st.borrow_mut().filters.enabled = "updated".into();
            this.mark(DIRTY_LIST);
        }));
        let this = app.clone();
        self.group_btn.clicked().connect(&SlotNoArgs::new(w, move || {
            let before = this.st.borrow().focused.clone();
            this.add_group(before);
        }));
        let this = app.clone();
        self.sort_btn.clicked().connect(&SlotNoArgs::new(w, move || {
            if !crate::dialogs::confirm(&this.window, "Sort A–Z", "Sort the load order alphabetically by pack file name (inside each group)?") {
                return;
            }
            let mut st = this.st.borrow_mut();
            let files: HashMap<String, String> = st.mods.iter().map(|m| (m.key.clone(), m.file.clone())).collect();
            st.update_active(|p| p.entries = profile_ops::sort_alpha(&p.entries, &files));
            drop(st);
            this.mark(DIRTY_LIST);
        }));
        let this = app.clone();
        self.top_btn.clicked().connect(&SlotNoArgs::new(w, move || {
            this.st.borrow_mut().update_active(|p| p.entries = profile_ops::enabled_to_top(&p.entries));
            this.mark(DIRTY_LIST);
        }));
        self.apply_density(&app.ctx.settings().list_density);
        let this = app.clone();
        self.density.activated().connect(&SlotOfInt::new(w, move |_| {
            let v = this.ml.density.current_data_0a().to_string().to_std_string();
            set_density(&this, &v);
        }));

        // Double-click a mod row (outside the check box) to enable / disable it.
        let this = app.clone();
        self.view.double_clicked().connect(&SlotOfQModelIndex::new(w, move |idx| {
            if idx.column() == COL_CHECK || this.st.borrow().game_running {
                return;
            }
            let key = key_of(&this.ml.model, &idx);
            if key.is_empty() || groups::is_separator_key(&key) || this.st.borrow().module(&key).is_none() {
                return;
            }
            this.st.borrow_mut().update_active(|p| profile_ops::toggle(&mut p.entries, &[key], None));
            this.mark(DIRTY_LIST | DIRTY_DETAILS | DIRTY_LAUNCH | DIRTY_HEADER);
        }));

        // Header click = sort by that column; clicking "#" again returns to load order.
        let this = app.clone();
        self.view.header().section_clicked().connect(&SlotOfInt::new(w, move |col| {
            let key = match col {
                COL_STATUS => Some(SortKey::Status),
                COL_TITLE => Some(SortKey::Title),
                COL_SOURCE => Some(SortKey::Source),
                COL_TYPE => Some(SortKey::Type),
                COL_SIZE => Some(SortKey::Size),
                COL_UPDATED => Some(SortKey::Updated),
                _ => None,
            };
            let mut st = this.st.borrow_mut();
            st.sort = match (key, st.sort) {
                (None, _) => (None, true),
                (Some(k), (Some(cur), asc)) if cur == k => (Some(k), !asc),
                (Some(k), _) => (Some(k), true),
            };
            drop(st);
            this.mark(DIRTY_LIST);
        }));

        // Check boxes (packs and group headers).
        let this = app.clone();
        self.model.item_changed().connect(&SlotOfQStandardItem::new(w, move |it| {
            if this.suppress.get() || it.column() != COL_CHECK || !it.is_checkable() {
                return;
            }
            let key = it.data_1a(role_key()).to_string().to_std_string();
            let on = it.check_state() == CheckState::Checked;
            if this.st.borrow().game_running {
                this.mark(DIRTY_LIST);
                return;
            }
            let selected = this.st.borrow().selected.clone();
            {
                let mut st = this.st.borrow_mut();
                if groups::is_separator_key(&key) {
                    st.update_active(|p| groups::toggle_group(&mut p.entries, &key, on));
                } else {
                    let keys = if selected.len() > 1 && selected.contains(&key) { selected } else { vec![key] };
                    st.update_active(|p| profile_ops::toggle(&mut p.entries, &keys, Some(on)));
                }
            }
            this.mark(DIRTY_LIST | DIRTY_DETAILS | DIRTY_LAUNCH | DIRTY_HEADER);
        }));

        // Drag & drop: note the rows Qt inserts; `flush_drop` applies the move from the pump,
        // after Qt has finished the drag.
        let this = app.clone();
        self.model.rows_inserted().connect(&SlotOfQModelIndexIntInt::new(w, move |parent, first, last| {
            if this.suppress.get() {
                return;
            }
            let mut pending = this.ml.pending_drop.borrow_mut();
            for r in first..=last {
                pending.push(QPersistentModelIndex::new_1a(&this.ml.model.index_3a(r, 0, parent)));
            }
        }));

        // Folding groups is remembered in the profile.
        for expand in [true, false] {
            let this = app.clone();
            let slot = SlotOfQModelIndex::new(w, move |idx| {
                if this.suppress.get() {
                    return;
                }
                let key = key_of(&this.ml.model, &idx);
                if groups::is_separator_key(&key) {
                    this.st.borrow_mut().update_active(|p| groups::set_collapsed(&mut p.entries, &key, !expand));
                }
            });
            if expand {
                self.view.expanded().connect(&slot);
            } else {
                self.view.collapsed().connect(&slot);
            }
        }

        let this = app.clone();
        self.view.selection_model().selection_changed().connect(&SlotOfQItemSelectionQItemSelection::new(w, move |_: cpp_core::Ref<QItemSelection>, _: cpp_core::Ref<QItemSelection>| {
            if this.suppress.get() {
                return;
            }
            this.ml.read_selection(&this);
            this.mark(DIRTY_DETAILS);
        }));

        let this = app.clone();
        self.view.custom_context_menu_requested().connect(&SlotOfQPoint::new(w, move |pos| this.ml.context_menu(&this, pos)));

        for (seq, dir) in [("Alt+Up", -1), ("Alt+Down", 1)] {
            let sc = QShortcut::from_q_key_sequence_q_object(&QKeySequence::from_q_string(&qs(seq)), &self.view);
            let this = app.clone();
            sc.activated().connect(&SlotNoArgs::new(w, move || {
                let keys = this.st.borrow().selected.clone();
                let moved = {
                    let mut st = this.st.borrow_mut();
                    let mut moved = false;
                    st.update_active(|p| moved = profile_ops::nudge(&mut p.entries, &keys, dir));
                    moved
                };
                if moved {
                    this.mark(DIRTY_LIST);
                }
            }));
            std::mem::forget(sc);
        }
    }

    /// Apply a row-size preset (unknown values fall back to "normal").
    pub unsafe fn apply_density(&self, value: &str) {
        let (v, _, row, plus, check, check_w, status_w) = DENSITIES.iter().copied().find(|d| d.0 == value).unwrap_or(DENSITIES[1]);
        self.view.set_style_sheet(&qs(format!(
            "QTreeView::item {{ min-height: {row}px; }} QTreeView::indicator {{ width: {check}px; height: {check}px; }}"
        )));
        let f = QFont::new_copy(self.view.font());
        f.set_point_size_f(self.base_point_size + plus);
        self.view.set_font(&f);
        let icon = (check - 2).max(12);
        self.view.set_icon_size(&qt_core::QSize::new_2a(icon, icon));
        let header = self.view.header();
        header.resize_section(COL_CHECK, check_w);
        header.resize_section(COL_STATUS, status_w);
        let i = self.density.find_data_1a(&QVariant::from_q_string(&qs(v)));
        self.density.set_current_index(i.max(0));
    }

    unsafe fn read_selection(&self, app: &Rc<App>) {
        let rows = self.view.selection_model().selected_rows_0a();
        let mut keys = Vec::new();
        for i in 0..rows.count() {
            let k = key_of(&self.model, &rows.at(i));
            if !k.is_empty() && k != MOVIE_HEADER_KEY {
                keys.push(k);
            }
        }
        let cur = self.view.current_index();
        let focused = if cur.is_valid() { Some(key_of(&self.model, &cur)).filter(|k| !k.is_empty() && k != MOVIE_HEADER_KEY) } else { None };
        let mut st = app.st.borrow_mut();
        st.selected = keys;
        st.focused = focused.or_else(|| st.selected.last().cloned());
    }

    /// Apply a finished drop. Qt's own move is only used to learn which rows were dropped and
    /// where: the keys and the row that now follows them give the new place, the profile is
    /// reordered with `groups::move_before`, and the list is rebuilt from the profile. That keeps
    /// odd drops (onto a pack, into another column) from leaving a half-moved model behind.
    pub unsafe fn flush_drop(&self, app: &Rc<App>) {
        let dropped: Vec<CppBox<QPersistentModelIndex>> = self.pending_drop.borrow_mut().drain(..).collect();
        let valid: Vec<&CppBox<QPersistentModelIndex>> = dropped.iter().filter(|p| p.is_valid()).collect();
        if dropped.is_empty() {
            return;
        }
        let row_key = |parent: &CppBox<QModelIndex>, row: i32| -> String {
            (0..self.model.column_count_1a(parent))
                .map(|c| self.model.index_3a(row, c, parent).data_1a(role_key()).to_string().to_std_string())
                .find(|k| !k.is_empty())
                .unwrap_or_default()
        };
        let mut keys: Vec<String> = Vec::new();
        for p in &valid {
            let k = row_key(&p.parent(), p.row());
            if !k.is_empty() && k != MOVIE_HEADER_KEY && !keys.contains(&k) {
                keys.push(k);
            }
        }
        if let Some(last) = valid.iter().max_by_key(|p| p.row()) {
            // Where to look for the row that follows the drop. Dropped under a pack (not a group)
            // means "after that pack".
            let mut parent = last.parent();
            let mut row = last.row() + 1;
            if parent.is_valid() && !groups::is_separator_key(&row_key(&parent.parent(), parent.row())) {
                row = parent.row() + 1;
                parent = parent.parent();
            }
            let mut anchor: Option<String> = None;
            loop {
                if row < self.model.row_count_1a(&parent) {
                    let k = row_key(&parent, row);
                    if k == MOVIE_HEADER_KEY {
                        break;
                    }
                    if !k.is_empty() && !keys.contains(&k) {
                        anchor = Some(k);
                        break;
                    }
                    row += 1;
                } else if parent.is_valid() {
                    // Past the end of a group: continue after it at the top level.
                    row = parent.row() + 1;
                    parent = parent.parent();
                } else {
                    break;
                }
            }
            if !keys.is_empty() {
                let mut st = app.st.borrow_mut();
                st.update_active(|p| {
                    groups::move_before(&mut p.entries, &keys, anchor.as_deref());
                });
                st.focused = keys.last().cloned();
                st.selected = keys.clone();
            }
        }
        app.mark(DIRTY_LIST | DIRTY_DETAILS);
    }

    unsafe fn context_menu(&self, app: &Rc<App>, pos: cpp_core::Ref<QPoint>) {
        let idx = self.view.index_at(pos);
        if !idx.is_valid() {
            return;
        }
        let key = key_of(&self.model, &idx);
        if key.is_empty() || key == MOVIE_HEADER_KEY {
            return;
        }
        let menu = QMenu::new();
        let running = app.st.borrow().game_running;
        let add = |label: &str, enabled: bool, f: Box<dyn Fn()>| {
            let a = menu.add_action_q_string(&qs(label));
            a.set_enabled(enabled);
            a.triggered().connect(&SlotNoArgs::new(&menu, move || f()));
        };
        if groups::is_separator_key(&key) {
            for (label, on) in [("Enable group", true), ("Disable group", false)] {
                let (this, k) = (app.clone(), key.clone());
                add(label, !running, Box::new(move || {
                    this.st.borrow_mut().update_active(|p| groups::toggle_group(&mut p.entries, &k, on));
                    this.mark(DIRTY_LIST | DIRTY_LAUNCH | DIRTY_HEADER);
                }));
            }
            menu.add_separator();
            let (this, k) = (app.clone(), key.clone());
            add("Rename…", true, Box::new(move || {
                let cur = this.st.borrow().active().entries.iter().find(|e| e.key == k).and_then(|e| e.label.clone()).unwrap_or_default();
                if let Some(l) = crate::dialogs::ask_text(&this.window, "Rename group", "Group name:", &cur) {
                    this.st.borrow_mut().update_active(|p| groups::rename_separator(&mut p.entries, &k, &l));
                    this.mark(DIRTY_LIST);
                }
            }));
            let (this, k) = (app.clone(), key.clone());
            add("Add group above…", true, Box::new(move || this.add_group(Some(k.clone()))));
            let (this, k) = (app.clone(), key.clone());
            add("Remove group (keeps its mods)", true, Box::new(move || {
                this.st.borrow_mut().update_active(|p| groups::remove_separator(&mut p.entries, &k));
                this.mark(DIRTY_LIST);
            }));
        } else {
            let selected = app.st.borrow().selected.clone();
            let keys = if selected.contains(&key) { selected } else { vec![key.clone()] };
            // A pack that is not installed cannot be enabled - its check box is not checkable and
            // double-click ignores it - so the menu must agree, or it writes a phantom "enabled"
            // entry that inflates the counts and never loads.
            let keys: Vec<String> = {
                let st = app.st.borrow();
                keys.into_iter().filter(|k| st.module(k).is_some()).collect()
            };
            let n = if keys.len() > 1 { format!(" ({})", keys.len()) } else { String::new() };
            for (label, on) in [("Enable", true), ("Disable", false)] {
                let (this, ks) = (app.clone(), keys.clone());
                add(&format!("{label}{n}"), !running && !ks.is_empty(), Box::new(move || {
                    this.st.borrow_mut().update_active(|p| profile_ops::toggle(&mut p.entries, &ks, Some(on)));
                    this.mark(DIRTY_LIST | DIRTY_LAUNCH | DIRTY_HEADER | DIRTY_DETAILS);
                }));
            }
            menu.add_separator();
            let (this, k) = (app.clone(), key.clone());
            add("Add group above…", true, Box::new(move || this.add_group(Some(k.clone()))));
            menu.add_separator();
            let m = app.st.borrow().module(&key).cloned();
            let ws = m.as_ref().and_then(|m| m.workshop_id.clone());
            let ws_ids = ws.clone();
            let this = app.clone();
            add("Open in Workshop", ws.is_some(), Box::new(move || {
                if let Some(id) = &ws {
                    crate::details::open_url(&format!("https://steamcommunity.com/sharedfiles/filedetails/?id={id}"));
                }
                let _ = &this;
            }));
            let this = app.clone();
            add("Force update from Steam", ws_ids.is_some() &&!running && !app.steam_busy.get(), Box::new(move || {
                if let Some(id) = &ws_ids {
                    this.force_update(vec![id.clone()]);
                }
            }));
            if let Some(nx) = m.as_ref().and_then(|m| m.nexus.clone()) {
                if let Some(id) = nx.mod_id {
                    add("Open on Nexus Mods", true, Box::new(move || crate::details::open_url(&tkmm_core::nexus::mod_page(id))));
                }
                let versions = menu.add_menu_q_string(&qs("Version"));
                for f in &nx.versions {
                    let label = version_label(f);
                    let a = versions.add_action_q_string(&qs(&label));
                    a.set_checkable(true);
                    a.set_checked(f.dir == nx.active.dir);
                    a.set_enabled(!running);
                    let (this, slot, dir) = (app.clone(), nx.slot.clone(), f.dir.clone());
                    a.triggered().connect(&SlotNoArgs::new(&menu, move || this.nexus_set_active(&slot, &dir)));
                }
                let (this, slot, dir, label) = (app.clone(), nx.slot.clone(), nx.active.dir.clone(), version_label(&nx.active));
                add("Delete this version…", !running, Box::new(move || this.nexus_remove(&slot, &dir, &label)));
            }
            let path = m.as_ref().map(|m| m.path.clone());
            add("Show file in Explorer", path.is_some(), Box::new(move || {
                if let Some(p) = &path {
                    crate::details::reveal(p);
                }
            }));
            menu.add_separator();
            let hidden = app.st.borrow().meta_of(&key).hidden;
            let (this, k) = (app.clone(), key.clone());
            add(if hidden { "Unhide" } else { "Hide" }, true, Box::new(move || {
                this.st.borrow_mut().set_meta(&k, |m| m.hidden = !hidden);
                this.mark(DIRTY_LIST | DIRTY_DETAILS);
            }));
            let (this, k) = (app.clone(), key.clone());
            add("Copy key", true, Box::new(move || this.clipboard_set(&k)));
            let n_missing = {
                let st = app.st.borrow();
                profile_ops::missing_keys(&st.active(), &st.mods).len()
            };
            if n_missing > 0 {
                menu.add_separator();
                let this = app.clone();
                add(&format!("Remove missing entries ({n_missing})…"), !running, Box::new(move || this.purge_missing()));
            }
        }
        menu.exec_1a_mut(&self.view.viewport().map_to_global_q_point(pos));
    }

    /// Rebuild the model from the active profile, keeping scroll position and selection.
    pub unsafe fn refresh(&self, app: &Rc<App>) {
        let st = app.st.borrow();
        let settings = app.ctx.settings();
        let cutoff = st.cutoff(settings.outdated_before);
        let profile = st.active();
        let have = st.dll_version();
        let f = st.filters.clone();
        let grouped = st.sort.0.is_none() && !f.is_narrowing();
        let can_drag = grouped && !st.game_running;
        let hidden_keys: HashSet<String> = st.meta.mods.iter().filter(|(_, m)| m.hidden).map(|(k, _)| k.clone()).collect();

        // Tags filter choices.
        let mut tags: Vec<String> = st.meta.mods.values().flat_map(|m| m.tags.clone()).collect();
        tags.sort();
        tags.dedup();
        self.tag.clear();
        self.tag.add_item_q_string_q_variant(&qs("Any tag"), &QVariant::from_q_string(&qs("")));
        for t in &tags {
            self.tag.add_item_q_string_q_variant(&qs(t), &QVariant::from_q_string(&qs(t)));
        }
        self.tag.set_visible(!tags.is_empty());
        if let Some(t) = &f.tag {
            let i = self.tag.find_data_1a(&QVariant::from_q_string(&qs(t)));
            self.tag.set_current_index(i.max(0));
        }
        for (c, v) in [(&self.source, &f.source), (&self.pack_type, &f.pack_type), (&self.enabled, &f.enabled), (&self.status, &f.status)] {
            let i = c.find_data_1a(&QVariant::from_q_string(&qs(v)));
            c.set_current_index(i.max(0));
        }
        self.hidden.set_checked(f.show_hidden);
        // Entries whose pack is not installed: hidden by default, but the user needs to know they
        // are there (they hold a place in the load order) and be able to clear them out.
        let missing_count = profile.entries.iter().filter(|e| !e.is_separator() && st.module(&e.key).is_none()).count();
        self.missing.set_visible(missing_count > 0);
        self.missing.set_text(&qs(format!("Show missing ({missing_count})")));
        self.missing.set_checked(f.show_missing);
        self.purge_btn.set_visible(missing_count > 0 && !st.game_running);
        self.purge_btn.set_text(&qs(format!("Remove missing ({missing_count})…")));
        self.group_btn.set_enabled(grouped);
        for b in [&self.sort_btn, &self.top_btn] {
            b.set_enabled(!st.game_running);
        }

        // Remember what the user was looking at.
        let scroll = self.view.vertical_scroll_bar().value();
        let selected: HashSet<String> = st.selected.iter().cloned().collect();
        let focused = st.focused.clone();

        self.model.remove_rows_2a(0, self.model.row_count_0a());
        // A drop at a column other than the first can add columns; keep the fixed set.
        self.model.set_column_count(COLUMN_COUNT);
        let q = f.search.trim().to_lowercase();
        let root = self.model.invisible_root_item();
        let mut order = 0;
        let mut shown = 0;
        let mut updated_count = 0;
        let mut current_group: Option<Ptr<QStandardItem>> = None;
        let mut flat_rows: Vec<(ProfileEntry, i32)> = Vec::new();
        let mut movie_rows: Vec<(ProfileEntry, i32)> = Vec::new();

        for e in &profile.entries {
            if e.is_separator() {
                if grouped {
                    let label = e.label.clone().unwrap_or_else(|| "Group".into());
                    let members: Vec<String> = groups::group_members(&profile.entries, &e.key).into_iter().filter(|k| st.module(k).is_some()).collect();
                    let on = members.iter().filter(|k| profile.entries.iter().any(|x| &x.key == *k && x.enabled)).count();
                    let head = item(&format!("{label}   ({on}/{} enabled)", members.len()));
                    head.set_data_2a(&QVariant::from_q_string(&qs(&e.key)), role_key());
                    let bold = QFont::new_copy(&head.font());
                    bold.set_bold(true);
                    head.set_font(&bold);
                    head.set_checkable(!members.is_empty());
                    head.set_check_state(match groups::group_state(&profile.entries, &e.key, |k| st.module(k).is_some()) {
                        GroupState::All => CheckState::Checked,
                        GroupState::Some => CheckState::PartiallyChecked,
                        _ => CheckState::Unchecked,
                    });
                    let mut flags = QFlags::from(ItemFlag::ItemIsEnabled) | ItemFlag::ItemIsSelectable | ItemFlag::ItemIsDropEnabled;
                    if can_drag {
                        flags = flags | ItemFlag::ItemIsDragEnabled;
                    }
                    if !members.is_empty() && !st.game_running {
                        flags = flags | ItemFlag::ItemIsUserCheckable;
                    }
                    head.set_flags(flags);
                    let head_ptr = head.as_ptr();
                    root.append_row_q_list_of_q_standard_item(&row_list(vec![head]));
                    self.view.set_first_column_spanned(head_ptr.row(), &QModelIndex::new(), true);
                    current_group = Some(head_ptr);
                }
                continue;
            }
            let m = st.module(&e.key);
            let is_movie = m.map(|m| m.pack_type == PackType::Movie).unwrap_or(false);
            if !is_movie {
                order += 1;
            }
            if hidden_keys.contains(&e.key) && !f.show_hidden {
                continue;
            }
            if m.is_none() && !f.show_missing {
                continue;
            }
            let ws = m.and_then(|m| st.ws_of(m));
            let stat = status::mod_status(m, ws, cutoff);
            let req = st.se_req(&e.key);
            let is_updated = e.enabled && profile_ops::updated_since(profile.last_played, m, ws);
            if is_updated {
                updated_count += 1;
            }
            if f.source != "all" && m.map(|m| source_value(m.source) != f.source).unwrap_or(true) {
                continue;
            }
            if f.pack_type != "all" && m.map(|m| (m.pack_type == PackType::Movie) != (f.pack_type == "movie")).unwrap_or(true) {
                continue;
            }
            match f.enabled.as_str() {
                "enabled" if !e.enabled => continue,
                "disabled" if e.enabled => continue,
                "updated" if !is_updated => continue,
                _ => {}
            }
            match f.status.as_str() {
                "pending" if stat.kind != StatusKind::Pending => continue,
                "old" if stat.kind != StatusKind::Old => continue,
                "se" if !req.required => continue,
                _ => {}
            }
            let meta = st.meta_of(&e.key);
            if let Some(t) = &f.tag {
                if !meta.tags.contains(t) {
                    continue;
                }
            }
            let title = st.title_of(&e.key);
            if !q.is_empty() && !title.to_lowercase().contains(&q) && !m.map(|m| m.file.to_lowercase().contains(&q)).unwrap_or(false) && !e.key.to_lowercase().contains(&q) {
                continue;
            }
            shown += 1;
            let row_order = if is_movie { 0 } else { order };
            if is_movie && grouped {
                movie_rows.push((e.clone(), row_order));
                continue;
            }
            if !grouped {
                flat_rows.push((e.clone(), row_order));
                continue;
            }
            let row = self.make_row(&st, e, row_order, &stat, &req, is_updated, &meta.tags, meta.hidden, can_drag, have.as_deref());
            match current_group {
                Some(g) => g.append_row_q_list_of_q_standard_item(&row),
                None => root.append_row_q_list_of_q_standard_item(&row),
            }
        }

        if !grouped {
            // Flat list, optionally sorted by a column.
            let (key, asc) = st.sort;
            if let Some(k) = key {
                let cached: HashMap<String, (StatusKind, bool, String, u64, u64)> = flat_rows
                    .iter()
                    .map(|(e, _)| {
                        let m = st.module(&e.key);
                        let ws = m.and_then(|m| st.ws_of(m));
                        let stat = status::mod_status(m, ws, cutoff).kind;
                        let upd = ws.map(|w| w.time_updated).filter(|t| *t > 0).or(m.map(|m| m.mtime)).unwrap_or(0);
                        (e.key.clone(), (stat, st.se_req(&e.key).required, st.title_of(&e.key), m.map(|m| m.size).unwrap_or(0), upd))
                    })
                    .collect();
                flat_rows.sort_by(|(a, ao), (b, bo)| {
                    let (x, y) = (&cached[&a.key], &cached[&b.key]);
                    let (ma, mb) = (st.module(&a.key), st.module(&b.key));
                    let o = match k {
                        SortKey::Status => x.0.rank().cmp(&y.0.rank()).then(y.1.cmp(&x.1)),
                        SortKey::Title => compare_pack_names(&x.2, &y.2),
                        SortKey::Source => ma.map(|m| source_value(m.source)).cmp(&mb.map(|m| source_value(m.source))),
                        SortKey::Type => ma.map(|m| m.pack_type == PackType::Movie).cmp(&mb.map(|m| m.pack_type == PackType::Movie)),
                        SortKey::Size => x.3.cmp(&y.3),
                        SortKey::Updated => x.4.cmp(&y.4),
                    };
                    let o = if asc { o } else { o.reverse() };
                    o.then(ao.cmp(bo))
                });
            }
            for (e, ord) in &flat_rows {
                let m = st.module(&e.key);
                let ws = m.and_then(|m| st.ws_of(m));
                let stat = status::mod_status(m, ws, cutoff);
                let req = st.se_req(&e.key);
                let meta = st.meta_of(&e.key);
                let upd = e.enabled && profile_ops::updated_since(profile.last_played, m, ws);
                let row = self.make_row(&st, e, *ord, &stat, &req, upd, &meta.tags, meta.hidden, false, have.as_deref());
                root.append_row_q_list_of_q_standard_item(&row);
            }
        }

        if !movie_rows.is_empty() {
            let head = item("Movie packs: always load after every mod pack; the engine fixes their order");
            head.set_data_2a(&QVariant::from_q_string(&qs(MOVIE_HEADER_KEY)), role_key());
            head.set_flags(QFlags::from(ItemFlag::ItemIsEnabled));
            head.set_foreground(&QBrush::from_q_color(&icons::amber()));
            let head_ptr = head.as_ptr();
            root.append_row_q_list_of_q_standard_item(&row_list(vec![head]));
            self.view.set_first_column_spanned(head_ptr.row(), &QModelIndex::new(), true);
            for (e, _) in &movie_rows {
                let m = st.module(&e.key);
                let ws = m.and_then(|m| st.ws_of(m));
                let stat = status::mod_status(m, ws, cutoff);
                let req = st.se_req(&e.key);
                let meta = st.meta_of(&e.key);
                let row = self.make_row(&st, e, 0, &stat, &req, false, &meta.tags, meta.hidden, false, have.as_deref());
                head_ptr.append_row_q_list_of_q_standard_item(&row);
            }
        }

        // Folding, drag & drop, sort indicator.
        for r in 0..root.row_count() {
            let top = root.child_1a(r);
            let key = top.data_1a(role_key()).to_string().to_std_string();
            let collapsed = profile.entries.iter().find(|e| e.key == key).map(|e| e.collapsed).unwrap_or(false);
            if top.has_children() {
                self.view.set_expanded(&top.index(), !collapsed);
            }
        }
        self.view.set_root_is_decorated(grouped);
        self.view.set_drag_enabled(can_drag);
        self.view.set_accept_drops(can_drag);
        self.view.set_drop_indicator_shown(can_drag);
        self.view.set_drag_drop_mode(if can_drag { DragDropMode::InternalMove } else { DragDropMode::NoDragDrop });
        let header = self.view.header();
        match st.sort {
            (Some(k), asc) => {
                header.set_sort_indicator_shown(true);
                header.set_sort_indicator(sort_column(k), if asc { SortOrder::AscendingOrder } else { SortOrder::DescendingOrder });
            }
            _ => header.set_sort_indicator_shown(false),
        }

        // Restore selection and scroll.
        let sel = self.view.selection_model();
        sel.clear();
        let mut current: Option<CppBox<QModelIndex>> = None;
        let mut visit = |it: Ptr<QStandardItem>| {
            let k = it.data_1a(role_key()).to_string().to_std_string();
            if selected.contains(&k) {
                sel.select_q_model_index_q_flags_selection_flag(&it.index(), qt_core::q_item_selection_model::SelectionFlag::Select | qt_core::q_item_selection_model::SelectionFlag::Rows);
            }
            if focused.as_deref() == Some(k.as_str()) {
                current = Some(it.index());
            }
        };
        for r in 0..root.row_count() {
            let top = root.child_1a(r);
            visit(top);
            for c in 0..top.row_count() {
                visit(top.child_1a(c));
            }
        }
        if let Some(c) = current {
            sel.set_current_index(&c, QFlags::from(qt_core::q_item_selection_model::SelectionFlag::NoUpdate));
        }
        self.view.vertical_scroll_bar().set_value(scroll);

        let total = profile.entries.iter().filter(|e| !e.is_separator()).count() - missing_count;
        let enabled = st.count_enabled(&profile);
        let missing_note = if missing_count > 0 { format!(" · {missing_count} missing") } else { String::new() };
        self.count.set_text(&qs(format!(
            "{enabled} enabled of {total} · {shown} shown{missing_note}{}",
            if grouped { "" } else { " · groups hidden while sorting or filtering" }
        )));
        self.updated_btn.set_visible(updated_count > 0 && f.enabled != "updated");
        self.updated_btn.set_text(&qs(format!("{updated_count} updated since last played")));
        let _ = update::ASSET_NAME;
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn make_row(
        &self,
        st: &crate::state::State,
        e: &ProfileEntry,
        order: i32,
        stat: &status::ModStatus,
        req: &status::SeRequirement,
        is_updated: bool,
        tags: &[String],
        hidden: bool,
        can_drag: bool,
        have: Option<&str>,
    ) -> CppBox<QListOfQStandardItem> {
        let m = st.module(&e.key);
        let running = st.game_running;
        let mut base = QFlags::from(ItemFlag::ItemIsEnabled) | ItemFlag::ItemIsSelectable;
        if can_drag {
            base = base | ItemFlag::ItemIsDragEnabled;
        }

        let check = item("");
        check.set_data_2a(&QVariant::from_q_string(&qs(&e.key)), role_key());
        check.set_checkable(m.is_some());
        check.set_check_state(if e.enabled { CheckState::Checked } else { CheckState::Unchecked });
        let mut cflags = base;
        if m.is_some() && !running {
            cflags = cflags | ItemFlag::ItemIsUserCheckable;
        }
        check.set_flags(cflags);

        let status_item = item(if req.required { "SE" } else { "" });
        status_item.set_icon(&icons::status_dot(stat.kind));
        let unmet = if e.enabled { status::se_unmet(req, st.active().dll, have) } else { None };
        let mut tip = format!("{}: {}", stat.kind.label(), stat.text);
        if req.required {
            tip.push_str(&format!("\n{}", status::source_text(req)));
            if let Some(err) = &req.error {
                tip.push_str(&format!("\n{err}"));
            }
            if let Some(u) = unmet {
                tip.push_str(&format!("\n{}", status::unmet_text(u, req, have)));
            }
            status_item.set_foreground(&QBrush::from_q_color(&if unmet.is_some() { icons::red() } else { icons::blue() }));
            let bold = QFont::new_copy(&status_item.font());
            bold.set_bold(true);
            status_item.set_font(&bold);
        }
        status_item.set_tool_tip(&qs(tip));

        let order_text = if order > 0 { order.to_string() } else { String::new() };
        let order_item = item(&order_text);
        order_item.set_text_alignment(QFlags::from(qt_core::AlignmentFlag::AlignRight) | qt_core::AlignmentFlag::AlignVCenter);

        let title_item = item(&st.title_of(&e.key));
        title_item.set_tool_tip(&qs(m.map(|m| m.file.clone()).unwrap_or_else(|| e.key.clone())));
        if m.is_none() {
            let f = QFont::new_copy(&title_item.font());
            f.set_strike_out(true);
            title_item.set_font(&f);
        }

        let mut flags: Vec<String> = Vec::new();
        if m.is_none() {
            flags.push("missing".into());
        }
        if is_updated {
            flags.push("updated".into());
        }
        if hidden {
            flags.push("hidden".into());
        }
        flags.extend(tags.iter().cloned());
        let flags_item = item(&flags.join(" · "));
        if is_updated {
            flags_item.set_foreground(&QBrush::from_q_color(&icons::green()));
        }

        let source_item = item(m.map(|m| st.source_label(m)).unwrap_or(""));
        if let Some(m) = m {
            source_item.set_tool_tip(&qs(&m.dir));
        }
        let type_item = item(m.map(|m| if m.pack_type == PackType::Movie { "MOVIE" } else { "mod" }).unwrap_or(""));
        if m.map(|m| m.pack_type == PackType::Movie).unwrap_or(false) {
            type_item.set_foreground(&QBrush::from_q_color(&icons::amber()));
        }
        let size_item = item(&m.map(|m| format_bytes(m.size)).unwrap_or_default());
        size_item.set_text_alignment(QFlags::from(qt_core::AlignmentFlag::AlignRight) | qt_core::AlignmentFlag::AlignVCenter);
        let upd = m.and_then(|m| st.ws_of(m)).map(|w| w.time_updated).filter(|t| *t > 0).or(m.map(|m| m.mtime)).unwrap_or(0);
        let updated_item = item(&format_date(upd));

        let items = vec![check, status_item, order_item, title_item, flags_item, source_item, type_item, size_item, updated_item];
        for it in items.iter().skip(1) {
            it.set_flags(base);
            if !e.enabled {
                it.set_foreground(&QBrush::from_q_color(&icons::grey()));
            }
        }
        row_list(items)
    }
}

/// "1.2 · Main file · installed 2026-09-22"
pub fn version_label(f: &tkmm_core::nexus::InstalledFile) -> String {
    let mut bits = Vec::new();
    if !f.version.is_empty() {
        bits.push(f.version.clone());
    }
    if !f.name.is_empty() && f.name != f.version {
        bits.push(f.name.clone());
    }
    if f.installed > 0 {
        bits.push(format!("installed {}", tkmm_core::fmt::format_date(f.installed)));
    }
    if bits.is_empty() { f.dir.clone() } else { bits.join(" · ") }
}

fn source_value(s: ModSource) -> &'static str {
    match s {
        ModSource::Workshop => "workshop",
        ModSource::Data => "data",
        ModSource::Folder => "folder",
        ModSource::Nexus => "nexus",
    }
}

fn sort_column(k: SortKey) -> i32 {
    match k {
        SortKey::Status => COL_STATUS,
        SortKey::Title => COL_TITLE,
        SortKey::Source => COL_SOURCE,
        SortKey::Type => COL_TYPE,
        SortKey::Size => COL_SIZE,
        SortKey::Updated => COL_UPDATED,
    }
}

impl App {
    pub unsafe fn add_group(self: &Rc<Self>, before: Option<String>) {
        let Some(label) = crate::dialogs::ask_text(&self.window, "Add group", "Group name:", "New group") else { return };
        self.st.borrow_mut().update_active(|p| {
            groups::add_separator(&mut p.entries, &label, before.as_deref());
        });
        self.mark(DIRTY_LIST);
    }
}

#[allow(dead_code)]
fn unused(_: QString) {}
