//! Native dialogs: messages, confirmations, text input and multi-line text views.

use qt_core::{qs, QBox, SlotNoArgs};
use qt_gui::QGuiApplication;
use qt_widgets::{
    q_dialog_button_box::StandardButton as BoxButton, q_line_edit::EchoMode, q_message_box::StandardButton, QDialog, QDialogButtonBox, QInputDialog, QLabel,
    QMainWindow, QMessageBox, QPlainTextEdit, QPushButton, QVBoxLayout, QWidget,
};

pub unsafe fn info(parent: &QBox<QMainWindow>, title: &str, text: &str) {
    message(parent, qt_widgets::q_message_box::Icon::Information, title, text, StandardButton::Ok.into());
}

pub unsafe fn error(parent: &QBox<QMainWindow>, text: &str) {
    message(parent, qt_widgets::q_message_box::Icon::Warning, "TK Mod Manager", text, StandardButton::Ok.into());
}

pub unsafe fn confirm(parent: &QBox<QMainWindow>, title: &str, text: &str) -> bool {
    message(parent, qt_widgets::q_message_box::Icon::Question, title, text, StandardButton::Yes | StandardButton::No) == StandardButton::Yes.to_int()
}

/// A message box shown in front of the main window; returns the clicked standard button.
unsafe fn message(
    parent: &QBox<QMainWindow>,
    icon: qt_widgets::q_message_box::Icon,
    title: &str,
    text: &str,
    buttons: qt_core::QFlags<StandardButton>,
) -> i32 {
    let mb = QMessageBox::from_q_widget(parent);
    mb.set_icon(icon);
    mb.set_window_title(&qs(title));
    mb.set_text(&qs(text));
    mb.set_standard_buttons(buttons);
    let r = exec_in_front(parent, &mb);
    mb.delete_later();
    r
}

/// Many questions are asked from the event pump after a wait (the launch questions come after the
/// Workshop check), and could open behind the main window, so the app looked stuck on "Checking
/// Workshop mods for updates…". Raise the owner, then the box above it. When another program has
/// the foreground, Windows won't hand it over; the taskbar button flashes instead.
unsafe fn exec_in_front(parent: &QBox<QMainWindow>, mb: &QBox<QMessageBox>) -> i32 {
    if parent.is_visible() {
        if parent.is_minimized() {
            parent.show_normal();
        }
        parent.raise();
    }
    mb.show();
    mb.raise();
    mb.activate_window();
    qt_widgets::QApplication::alert_2a(mb, 0);
    mb.exec()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UpdateChoice {
    Install,
    Skip,
    Later,
}

/// Startup update offer: Install / Skip this version / Not now (Esc and closing = Not now).
pub unsafe fn ask_update(parent: &QBox<QMainWindow>, title: &str, text: &str) -> UpdateChoice {
    use qt_widgets::q_message_box::{ButtonRole, Icon};
    let mb = QMessageBox::from_q_widget(parent);
    mb.set_icon(Icon::Question);
    mb.set_window_title(&qs(title));
    mb.set_text(&qs(text));
    let install = mb.add_button_q_string_button_role(&qs(t!("dialog-install")), ButtonRole::AcceptRole);
    mb.add_button_q_string_button_role(&qs(t!("dialog-skip-version")), ButtonRole::DestructiveRole);
    let later = mb.add_button_q_string_button_role(&qs(t!("dialog-not-now")), ButtonRole::RejectRole);
    mb.set_default_button_q_push_button(install);
    mb.set_escape_button_q_abstract_button(later);
    exec_in_front(parent, &mb);
    let clicked = mb.clicked_button();
    let choice = if clicked.is_null() {
        UpdateChoice::Later
    } else {
        match mb.button_role(&clicked) {
            ButtonRole::AcceptRole => UpdateChoice::Install,
            ButtonRole::DestructiveRole => UpdateChoice::Skip,
            _ => UpdateChoice::Later,
        }
    };
    mb.delete_later();
    choice
}

/// Release notes trimmed to fit a message box (the full text stays in the tooltip).
pub fn clip_notes(notes: &str) -> String {
    const MAX_LINES: usize = 15;
    let lines: Vec<&str> = notes.trim().lines().collect();
    if lines.len() <= MAX_LINES {
        return lines.join("\n");
    }
    format!("{}\n…", lines[..MAX_LINES].join("\n"))
}

/// Pick one of `items`; None when cancelled.
pub unsafe fn choose(parent: &QBox<QMainWindow>, title: &str, label: &str, items: &[String]) -> Option<String> {
    let list = qt_core::QListOfQString::new_0a();
    for i in items {
        list.append_q_string(&qs(i));
    }
    let mut ok = false;
    let s = QInputDialog::get_item_7a(parent, &qs(title), &qs(label), &list, 0, false, &mut ok).to_std_string();
    ok.then_some(s)
}

/// Single-line text input; None when cancelled or empty.
pub unsafe fn ask_text(parent: &QBox<QMainWindow>, title: &str, label: &str, initial: &str) -> Option<String> {
    let mut ok = false;
    let s = QInputDialog::get_text_6a(parent, &qs(title), &qs(label), EchoMode::Normal, &qs(initial), &mut ok).to_std_string();
    let s = s.trim().to_string();
    (ok && !s.is_empty()).then_some(s)
}

fn dialog(parent: &QBox<QMainWindow>, title: &str, width: i32, height: i32) -> (QBox<QDialog>, QBox<QVBoxLayout>) {
    unsafe {
        let d = QDialog::new_1a(parent);
        d.set_window_title(&qs(title));
        d.resize_2a(width, height);
        let l = QVBoxLayout::new_1a(&d);
        (d, l)
    }
}

/// Read-only text with a Copy button.
pub unsafe fn show_text(parent: &QBox<QMainWindow>, title: &str, intro: &str, text: &str) {
    let (d, l) = dialog(parent, title, 760, 460);
    if !intro.is_empty() {
        let lab = QLabel::from_q_string(&qs(intro));
        lab.set_word_wrap(true);
        l.add_widget(&lab);
    }
    let edit = QPlainTextEdit::new();
    edit.set_read_only(true);
    edit.set_plain_text(&qs(text));
    let f = qt_gui::QFont::new_copy(edit.font());
    f.set_family(&qs("Consolas"));
    edit.set_font(&f);
    l.add_widget(&edit);
    let buttons = QDialogButtonBox::new();
    let copy = QPushButton::from_q_string(&qs(t!("dialog-copy")));
    buttons.add_button_q_abstract_button_button_role(&copy, qt_widgets::q_dialog_button_box::ButtonRole::ActionRole);
    let close = buttons.add_button_standard_button(BoxButton::Close);
    l.add_widget(&buttons);
    let t = text.to_string();
    copy.clicked().connect(&SlotNoArgs::new(&d, move || QGuiApplication::clipboard().set_text_1a(&qs(&t))));
    let dp = d.as_ptr();
    close.clicked().connect(&SlotNoArgs::new(&d, move || dp.accept()));
    d.exec();
}

/// Multi-line input (paste an export); None when cancelled.
pub unsafe fn ask_multiline(parent: &QBox<QMainWindow>, title: &str, intro: &str, ok_label: &str) -> Option<String> {
    multiline(parent, title, intro, ok_label, None)
}

/// Edit `initial` as plain text (monospace); None when cancelled.
pub unsafe fn edit_text(parent: &QBox<QMainWindow>, title: &str, intro: &str, initial: &str) -> Option<String> {
    multiline(parent, title, intro, &t!("dialog-ok"), Some(initial))
}

unsafe fn multiline(parent: &QBox<QMainWindow>, title: &str, intro: &str, ok_label: &str, initial: Option<&str>) -> Option<String> {
    let (d, l) = dialog(parent, title, 760, 460);
    let lab = QLabel::from_q_string(&qs(intro));
    lab.set_word_wrap(true);
    l.add_widget(&lab);
    let edit = QPlainTextEdit::new();
    if let Some(text) = initial {
        edit.set_plain_text(&qs(text));
        let f = qt_gui::QFont::new_copy(edit.font());
        f.set_family(&qs("Consolas"));
        edit.set_font(&f);
    }
    l.add_widget(&edit);
    let buttons = QDialogButtonBox::new();
    let ok = buttons.add_button_q_string_button_role(&qs(ok_label), qt_widgets::q_dialog_button_box::ButtonRole::AcceptRole);
    let cancel = buttons.add_button_standard_button(BoxButton::Cancel);
    l.add_widget(&buttons);
    let dp = d.as_ptr();
    ok.clicked().connect(&SlotNoArgs::new(&d, move || dp.accept()));
    let dp = d.as_ptr();
    cancel.clicked().connect(&SlotNoArgs::new(&d, move || dp.reject()));
    if d.exec() == 1 {
        Some(edit.to_plain_text().to_std_string())
    } else {
        None
    }
}

/// A dialog whose body the caller fills (used for the crash details and sync results).
pub unsafe fn custom(parent: &QBox<QMainWindow>, title: &str, width: i32, height: i32, fill: impl FnOnce(&QBox<QVBoxLayout>, &QBox<QDialog>)) {
    let (d, l) = dialog(parent, title, width, height);
    fill(&l, &d);
    let buttons = QDialogButtonBox::new();
    let close = buttons.add_button_standard_button(BoxButton::Close);
    l.add_widget(&buttons);
    let dp = d.as_ptr();
    close.clicked().connect(&SlotNoArgs::new(&d, move || dp.accept()));
    d.exec();
}

#[allow(dead_code)]
pub fn as_widget(w: &QBox<QMainWindow>) -> cpp_core::Ptr<QWidget> {
    unsafe { w.as_ptr().static_upcast() }
}
