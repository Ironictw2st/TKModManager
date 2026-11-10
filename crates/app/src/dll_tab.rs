//! The DLLs tab: the DLLs the manager can inject, as cards on the left, and the selected one's
//! details on the right. For now the only card is the script extender: whether this profile
//! uses it, its installed versions (the box Settings used to hold) and its features.

use crate::app::{App, DIRTY_DETAILS, DIRTY_LAUNCH, DIRTY_LIST};
use crate::details::{bold, colored, label};
use crate::icons;
use crate::se_settings::Features;
use qt_core::{qs, QBox, SlotOfBool};
use qt_widgets::{QCheckBox, QGroupBox, QLabel, QListWidget, QListWidgetItem, QScrollArea, QSplitter, QVBoxLayout, QWidget};
use std::rc::Rc;

/// Card lists: roomy rows, and a quiet selection (the Windows accent can be red, which reads as
/// an error on a whole card).
pub const CARD_STYLE: &str = "QListWidget::item { padding: 8px 6px; } \
    QListWidget::item:selected { background: palette(midlight); color: palette(window-text); border-left: 3px solid palette(highlight); }";

pub struct DllTabUi {
    pub root: QBox<QSplitter>,
    cards: QBox<QListWidget>,
    status: QBox<QLabel>,
    use_se: QBox<QCheckBox>,
    pub features: Rc<Features>,
}

impl DllTabUi {
    /// `versions` is Settings' script-extender box (install, update, pin), shown here.
    pub fn build(versions: &QBox<QGroupBox>) -> DllTabUi {
        unsafe {
            let root = QSplitter::new();
            let cards = QListWidget::new_0a();
            cards.set_minimum_width(180);
            cards.set_maximum_width(300);
            cards.set_style_sheet(&qs(CARD_STYLE));
            root.add_widget(&cards);
            let item = QListWidgetItem::from_q_string(&qs(t!("dlls-se-name")));
            cards.add_item_q_list_widget_item(item.into_ptr());
            cards.set_current_row_1a(0);

            let scroll = QScrollArea::new_0a();
            scroll.set_widget_resizable(true);
            scroll.set_frame_shape(qt_widgets::q_frame::Shape::NoFrame);
            root.add_widget(&scroll);
            root.set_stretch_factor(0, 0);
            root.set_stretch_factor(1, 1);
            root.set_children_collapsible(false);
            let body = QWidget::new_0a();
            let v = QVBoxLayout::new_1a(&body);
            v.set_contents_margins_4a(8, 0, 8, 0);
            v.set_spacing(10);
            let title = label(&t!("dlls-se-name"));
            bold(&title);
            let f = qt_gui::QFont::new_copy(title.font());
            f.set_point_size(f.point_size() + 2);
            title.set_font(&f);
            v.add_widget(&title);
            v.add_widget(&label(&t!("dlls-se-about")));
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
            scroll.set_widget(&body);
            DllTabUi { root, cards, status, use_se, features }
        }
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
        if let Some(item) = self.cards.item(0).as_ref() {
            let sub = match (&selected, on) {
                (Some(v), true) => t!("dlls-card-on", version = v),
                (Some(v), false) => t!("dlls-card-off", version = v),
                (None, _) => t!("dlls-card-not-installed"),
            };
            item.set_text(&qs(format!("{}\n{sub}", t!("dlls-se-name"))));
        }
        app.settings.refresh_dll_box(app);
        self.features.load(app);
    }
}
