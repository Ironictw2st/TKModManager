//! Native dialogs: messages, confirmations, text input and multi-line text views.

use qt_core::{qs, QBox, SlotNoArgs};
use qt_gui::QGuiApplication;
use qt_widgets::{
    q_dialog_button_box::StandardButton as BoxButton, q_line_edit::EchoMode, q_message_box::StandardButton, QDialog, QDialogButtonBox, QInputDialog, QLabel,
    QMainWindow, QMessageBox, QPlainTextEdit, QPushButton, QVBoxLayout, QWidget,
};

pub unsafe fn info(parent: &QBox<QMainWindow>, title: &str, text: &str) {
    QMessageBox::information_q_widget2_q_string(parent, &qs(title), &qs(text));
}

pub unsafe fn error(parent: &QBox<QMainWindow>, text: &str) {
    QMessageBox::warning_q_widget2_q_string(parent, &qs("TK Mod Manager"), &qs(text));
}

pub unsafe fn confirm(parent: &QBox<QMainWindow>, title: &str, text: &str) -> bool {
    QMessageBox::question_q_widget2_q_string(parent, &qs(title), &qs(text)) == StandardButton::Yes
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
    let copy = QPushButton::from_q_string(&qs("Copy to clipboard"));
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
    let (d, l) = dialog(parent, title, 760, 460);
    let lab = QLabel::from_q_string(&qs(intro));
    lab.set_word_wrap(true);
    l.add_widget(&lab);
    let edit = QPlainTextEdit::new();
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
