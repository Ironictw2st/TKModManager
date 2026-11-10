//! The DLLs tab: the DLLs that add to the game, as cards on the left (like a workshop), and the
//! selected one's page on the right.
//! - Script extender: whether this profile uses it, its versions (the box Settings used to hold)
//!   and its features.
//! - Three Kingdoms Coop: Thom-Ernst's multiplayer DLL, installed by its own Setup (the Coop box
//!   Settings used to hold).

use crate::app::{App, DIRTY_DETAILS, DIRTY_LAUNCH, DIRTY_LIST};
use crate::details::{bold, colored, label};
use crate::icons;
use crate::se_settings::Features;
use qt_core::{qs, QBox, SlotOfBool, SlotOfInt};
use qt_gui::QBrush;
use qt_widgets::{QCheckBox, QGroupBox, QLabel, QListWidget, QListWidgetItem, QScrollArea, QSplitter, QStackedWidget, QVBoxLayout, QWidget};
use std::rc::Rc;

/// Card lists: roomy rows, and a quiet selection (the Windows accent can be red, which reads as
/// an error on a whole card).
pub const CARD_STYLE: &str = "QListWidget::item { padding: 8px 6px; } \
    QListWidget::item:selected { background: palette(midlight); color: palette(window-text); border-left: 3px solid palette(highlight); }";

/// The cards, in list order.
#[derive(Clone, Copy)]
pub enum Card {
    ScriptExtender = 0,
    Coop = 1,
}

pub struct DllTabUi {
    pub root: QBox<QSplitter>,
    cards: QBox<QListWidget>,
    pages: QBox<QStackedWidget>,
    status: QBox<QLabel>,
    use_se: QBox<QCheckBox>,
    pub features: Rc<Features>,
}

/// A card's page: scrolls, starts with the DLL's name, who made it and what it does.
unsafe fn card_page(pages: &QBox<QStackedWidget>, name: &str, author: &str, about: &str) -> QBox<QVBoxLayout> {
    let scroll = QScrollArea::new_0a();
    scroll.set_widget_resizable(true);
    scroll.set_frame_shape(qt_widgets::q_frame::Shape::NoFrame);
    let body = QWidget::new_0a();
    let v = QVBoxLayout::new_1a(&body);
    v.set_contents_margins_4a(8, 0, 8, 0);
    v.set_spacing(10);
    let title = label(name);
    bold(&title);
    let f = qt_gui::QFont::new_copy(title.font());
    f.set_point_size(f.point_size() + 2);
    title.set_font(&f);
    v.add_widget(&title);
    let by = label(author);
    colored(&by, &icons::grey());
    v.add_widget(&by);
    v.add_widget(&label(about));
    scroll.set_widget(&body);
    pages.add_widget(&scroll);
    v
}

impl DllTabUi {
    /// `versions` and `coop` are Settings' script-extender and Coop boxes, shown here.
    pub fn build(versions: &QBox<QGroupBox>, coop: &QBox<QGroupBox>) -> DllTabUi {
        unsafe {
            let root = QSplitter::new();
            let cards = QListWidget::new_0a();
            cards.set_minimum_width(200);
            cards.set_maximum_width(300);
            cards.set_style_sheet(&qs(CARD_STYLE));
            root.add_widget(&cards);
            for name in [t!("dlls-se-name"), t!("dlls-coop-name")] {
                cards.add_item_q_list_widget_item(QListWidgetItem::from_q_string(&qs(name)).into_ptr());
            }
            let pages = QStackedWidget::new_0a();
            root.add_widget(&pages);
            root.set_stretch_factor(0, 0);
            root.set_stretch_factor(1, 1);
            root.set_children_collapsible(false);

            // Script extender.
            let v = card_page(&pages, &t!("dlls-se-name"), &t!("dlls-se-author"), &t!("dlls-se-about"));
            let status = label("");
            v.add_widget(&status);
            let use_se = QCheckBox::from_q_string(&qs(t!("dlls-se-use")));
            use_se.set_tool_tip(&qs(t!("launch-panel-se-tip")));
            v.add_widget(&use_se);
            versions.set_title(&qs(t!("dlls-se-versions")));
            v.add_widget(versions);
            let features = Rc::new(Features::build());
            v.add_widget(&features.root);
            v.add_stretch_1a(1);

            // Three Kingdoms Coop.
            let v = card_page(&pages, &t!("dlls-coop-name"), &t!("dlls-coop-author"), &t!("dlls-coop-about"));
            v.add_widget(coop);
            v.add_stretch_1a(1);

            cards.set_current_row_1a(0);
            let pp = pages.as_ptr();
            cards.current_row_changed().connect(&SlotOfInt::new(&root, move |i| {
                if i >= 0 {
                    pp.set_current_index(i)
                }
            }));
            DllTabUi { root, cards, pages, status, use_se, features }
        }
    }

    pub unsafe fn show_card(&self, card: Card) {
        self.cards.set_current_row_1a(card as i32);
        self.pages.set_current_index(card as i32);
    }

    pub unsafe fn connect(&self, app: &Rc<App>) {
        let this = app.clone();
        self.use_se.clicked().connect(&SlotOfBool::new(&app.window, move |on| {
            if this.suppress.get() {
                return;
            }
            this.st.borrow_mut().update_active(|p| p.dll = on);
            this.mark(DIRTY_LAUNCH | DIRTY_LIST | DIRTY_DETAILS);
        }));
        self.features.connect(app);
    }

    pub unsafe fn refresh(&self, app: &Rc<App>) {
        let (on, running, selected, installed) = {
            let st = app.st.borrow();
            let d = st.dll.as_ref();
            (st.active().dll, st.game_running, d.and_then(|d| d.selected.as_ref()).map(|s| s.version.clone()), d.map(|d| !d.installed.is_empty()).unwrap_or(false))
        };
        self.use_se.set_checked(on);
        self.use_se.set_enabled(!running);
        let (text, color) = match (&selected, on) {
            (Some(v), true) => (t!("dlls-se-status-on", version = v), icons::green()),
            (Some(v), false) => (t!("dlls-se-status-off", version = v), icons::grey()),
            (None, _) if installed => (t!("dlls-se-status-no-match"), icons::red()),
            (None, _) => (t!("dlls-se-status-none"), icons::amber()),
        };
        self.status.set_text(&qs(&text));
        colored(&self.status, &color);
        let se_sub = match (&selected, on) {
            (Some(v), true) => t!("dlls-card-on", version = v),
            (Some(v), false) => t!("dlls-card-off", version = v),
            (None, _) => t!("dlls-card-not-installed"),
        };
        let (coop_sub, coop_color) = {
            let st = app.st.borrow();
            match (&st.coop, st.coop_supported()) {
                (_, false) => (t!("dlls-card-steam-only"), None),
                (Some(c), true) if c.active() => (t!("dlls-card-installed", version = c.version.clone().unwrap_or_else(|| "?".into())), None),
                (Some(_), true) => (t!("dlls-card-needs-repair"), Some(icons::red())),
                (None, true) => (t!("dlls-card-not-installed"), None),
            }
        };
        for (i, name, sub, color) in [(0, t!("dlls-se-name"), se_sub, None), (1, t!("dlls-coop-name"), coop_sub, coop_color)] {
            if let Some(item) = self.cards.item(i).as_ref() {
                item.set_text(&qs(format!("{name}\n{sub}")));
                if let Some(c) = color {
                    item.set_foreground(&QBrush::from_q_color(&c));
                }
            }
        }
        app.settings.refresh_dll_box(app);
        app.settings.refresh_coop_box(app);
        self.features.load(app);
    }
}
