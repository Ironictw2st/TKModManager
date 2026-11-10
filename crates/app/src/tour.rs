//! The guided tour: a small card next to each main area of the window, with an outline around
//! the area, Back / Next / End tour. Plain child widgets of the main window (ritual can't
//! override paint events), placed again on every pump tick so they follow resizes.

use crate::app::{App, PAGE_CONFLICTS, PAGE_DLLS, PAGE_LOGS, PAGE_MODS};
use crate::details::{bold, label};
use cpp_core::Ptr;
use qt_core::{qs, QBox, QPoint, SlotNoArgs, WidgetAttribute};
use qt_widgets::{QFrame, QHBoxLayout, QLabel, QPushButton, QVBoxLayout, QWidget};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Copy)]
enum Target {
    ModList,
    Profile,
    Launch,
    Tabs,
    Conflicts,
    Troubleshooting,
    Settings,
}

struct Step {
    /// The page shown for this step (None: whatever is open).
    page: Option<i32>,
    target: Target,
    title: String,
    text: String,
}

fn steps() -> Vec<Step> {
    vec![
        Step { page: Some(PAGE_MODS), target: Target::ModList, title: t!("tour-mods-title"), text: t!("tour-mods-text") },
        Step { page: Some(PAGE_MODS), target: Target::Profile, title: t!("tour-profile-title"), text: t!("tour-profile-text") },
        Step { page: Some(PAGE_MODS), target: Target::Launch, title: t!("tour-play-title"), text: t!("tour-play-text") },
        Step { page: Some(PAGE_DLLS), target: Target::Tabs, title: t!("tour-dlls-title"), text: t!("tour-dlls-text") },
        Step { page: Some(PAGE_CONFLICTS), target: Target::Conflicts, title: t!("tour-conflicts-title"), text: t!("tour-conflicts-text") },
        Step { page: Some(PAGE_LOGS), target: Target::Troubleshooting, title: t!("tour-trouble-title"), text: t!("tour-trouble-text") },
        Step { page: Some(PAGE_MODS), target: Target::Settings, title: t!("tour-settings-title"), text: t!("tour-settings-text") },
    ]
}

pub struct Tour {
    steps: Vec<Step>,
    step: Cell<usize>,
    card: QBox<QFrame>,
    outline: QBox<QFrame>,
    title: QBox<QLabel>,
    text: QBox<QLabel>,
    count: QBox<QLabel>,
    back: QBox<QPushButton>,
    next: QBox<QPushButton>,
}

const CARD_WIDTH: i32 = 360;

unsafe fn target_widget(app: &App, t: Target) -> Ptr<QWidget> {
    match t {
        Target::ModList => app.ml.view.as_ptr().static_upcast(),
        Target::Profile => app.profile_combo.as_ptr().static_upcast(),
        Target::Launch => app.launch_layout.parent_widget().as_ptr(),
        Target::Tabs => app.tabs.as_ptr().static_upcast(),
        Target::Conflicts => app.conflicts.root.as_ptr().static_upcast(),
        Target::Troubleshooting => app.logs.root.as_ptr().static_upcast(),
        Target::Settings => app.settings_btn.as_ptr().static_upcast(),
    }
}

/// Start (or restart) the tour at its first step.
pub unsafe fn start(app: &Rc<App>) {
    end(app);
    let window: Ptr<QWidget> = app.window.as_ptr().static_upcast();
    let outline = QFrame::new_1a(window);
    outline.set_object_name(&qs("tourOutline"));
    outline.set_style_sheet(&qs("QFrame#tourOutline { border: 3px solid palette(highlight); border-radius: 5px; background: transparent; }"));
    outline.set_attribute_1a(WidgetAttribute::WATransparentForMouseEvents);
    let card = QFrame::new_1a(window);
    card.set_object_name(&qs("tourCard"));
    card.set_style_sheet(&qs("QFrame#tourCard { background: palette(window); border: 2px solid palette(highlight); border-radius: 8px; }"));
    card.set_fixed_width(CARD_WIDTH);
    let v = QVBoxLayout::new_1a(&card);
    v.set_contents_margins_4a(14, 12, 14, 12);
    let count = label("");
    v.add_widget(&count);
    let title = label("");
    bold(&title);
    let f = qt_gui::QFont::new_copy(title.font());
    f.set_point_size(f.point_size() + 1);
    title.set_font(&f);
    v.add_widget(&title);
    let text = label("");
    v.add_widget(&text);
    let row = QHBoxLayout::new_0a();
    v.add_layout_1a(&row);
    let end_btn = QPushButton::from_q_string(&qs(t!("tour-end")));
    row.add_widget(&end_btn);
    row.add_stretch_1a(1);
    let back = QPushButton::from_q_string(&qs(t!("tour-back")));
    let next = QPushButton::from_q_string(&qs(t!("tour-next")));
    next.set_default(true);
    row.add_widget(&back);
    row.add_widget(&next);
    let tour = Rc::new(Tour { steps: steps(), step: Cell::new(0), card, outline, title, text, count, back, next });
    let a = app.clone();
    end_btn.clicked().connect(&SlotNoArgs::new(&tour.card, move || end(&a)));
    let a = app.clone();
    tour.back.clicked().connect(&SlotNoArgs::new(&tour.card, move || go(&a, -1)));
    let a = app.clone();
    tour.next.clicked().connect(&SlotNoArgs::new(&tour.card, move || go(&a, 1)));
    *app.tour.borrow_mut() = Some(tour.clone());
    show_step(app, &tour);
    tour.outline.show();
    tour.card.show();
}

unsafe fn go(app: &Rc<App>, delta: i32) {
    let Some(tour) = app.tour.borrow().clone() else { return };
    let i = tour.step.get() as i32 + delta;
    if i < 0 {
        return;
    }
    if i as usize >= tour.steps.len() {
        end(app);
        return;
    }
    tour.step.set(i as usize);
    show_step(app, &tour);
}

unsafe fn show_step(app: &Rc<App>, tour: &Rc<Tour>) {
    let i = tour.step.get();
    let s = &tour.steps[i];
    if let Some(p) = s.page {
        if app.page() != p {
            app.set_page(p);
        }
    }
    tour.count.set_text(&qs(t!("tour-count", step = i + 1, total = tour.steps.len())));
    tour.title.set_text(&qs(&s.title));
    tour.text.set_text(&qs(&s.text));
    tour.back.set_enabled(i > 0);
    tour.next.set_text(&qs(if i + 1 == tour.steps.len() { t!("tour-done") } else { t!("tour-next") }));
    place(app);
}

/// Put the outline around this step's area and the card next to it (pump tick and each step).
pub unsafe fn place(app: &Rc<App>) {
    let Some(tour) = app.tour.borrow().clone() else { return };
    let s = &tour.steps[tour.step.get()];
    let target = target_widget(app, s.target);
    if target.is_null() {
        return;
    }
    let window: Ptr<QWidget> = app.window.as_ptr().static_upcast();
    let tl = target.map_to_q_widget_q_point(window, &QPoint::new_2a(0, 0));
    let (x, y, w, h) = (tl.x(), tl.y(), target.width(), target.height());
    let pad = 4;
    tour.outline.set_geometry_4a(x - pad, y - pad, w + 2 * pad, h + 2 * pad);
    tour.outline.raise();
    tour.card.adjust_size();
    let (cw, ch) = (tour.card.width(), tour.card.height());
    let (ww, wh) = (app.window.width(), app.window.height());
    let gap = 12;
    // Big areas: the card sits inside them, near the top. Small ones: below, else above.
    let (cx, cy) = if h > wh / 2 {
        (x + (w - cw) / 2, y + 40)
    } else if y + h + pad + gap + ch < wh {
        (x, y + h + pad + gap)
    } else {
        (x, y - pad - gap - ch)
    };
    let cx = cx.clamp(8, (ww - cw - 8).max(8));
    let cy = cy.clamp(8, (wh - ch - 8).max(8));
    tour.card.move_2a(cx, cy);
    tour.card.raise();
}

pub unsafe fn end(app: &Rc<App>) {
    if let Some(t) = app.tour.borrow_mut().take() {
        t.card.hide();
        t.outline.hide();
        t.card.delete_later();
        t.outline.delete_later();
    }
}
