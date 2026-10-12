//! Choosing the UI language: the first-start picker, the Settings combo, and Qt's own
//! translations (standard buttons, calendar). The text itself comes from `tkmm_core::i18n`.

use qt_core::{qs, QBox, QVariant, SlotNoArgs};
use qt_widgets::{QComboBox, QDialog, QDialogButtonBox, QLabel, QVBoxLayout};
use tkmm_core::{i18n, Ctx};

/// Every available language, shown by its own name, with the code as item data.
pub unsafe fn combo() -> QBox<QComboBox> {
    let c = QComboBox::new_0a();
    for (code, name) in i18n::available() {
        c.add_item_q_string_q_variant(&qs(name), &QVariant::from_q_string(&qs(code)));
    }
    c
}

/// Load the saved language, asking for one first if none was chosen yet. Runs before any
/// window exists, so every widget is built in the chosen language.
pub unsafe fn choose_and_load(ctx: &Ctx) {
    let mut s = ctx.settings();
    if s.language.is_empty() {
        s.language = if i18n::available().len() > 1 { ask().unwrap_or_else(i18n::system_default) } else { i18n::SOURCE.to_string() };
        let _ = ctx.set_settings(s.clone());
    }
    i18n::init(&s.language);
    load_qt_translations(&i18n::current());
}

/// The first-start dialog. Not translated beyond the language names: the user can't read the
/// app yet. Starts on the language that matches Windows.
unsafe fn ask() -> Option<String> {
    let d = QDialog::new_0a();
    d.set_window_title(&qs("TK Mod Manager"));
    let v = QVBoxLayout::new_1a(&d);
    let def = i18n::system_default();
    // The prompt in the suggested language, with English underneath when that differs.
    i18n::init(&def);
    let mut prompt = t!("language-choose");
    if def != i18n::SOURCE {
        i18n::init(i18n::SOURCE);
        prompt = format!("{prompt}\n{}", t!("language-choose"));
    }
    v.add_widget(QLabel::from_q_string(&qs(prompt)).into_ptr());
    let c = combo();
    let i = c.find_data_1a(&QVariant::from_q_string(&qs(&def)));
    c.set_current_index(i.max(0));
    v.add_widget(&c);
    let buttons = QDialogButtonBox::from_q_flags_standard_button(qt_widgets::q_dialog_button_box::StandardButton::Ok.into());
    v.add_widget(&buttons);
    let dp = d.as_ptr();
    buttons.accepted().connect(&SlotNoArgs::new(&d, move || dp.accept()));
    d.set_minimum_width(320);
    let chosen = if d.exec() == 1 { Some(c.current_data_0a().to_string().to_std_string()) } else { None };
    d.delete_later();
    chosen.filter(|c| !c.is_empty())
}

/// Qt's own strings (Yes / No / Cancel, the date picker) from `translations\qtbase_<code>.qm`
/// next to the exe, which release.ps1 copies from Craft's Qt for each shipped language.
unsafe fn load_qt_translations(code: &str) {
    qt_core::QLocale::set_default(&qt_core::QLocale::from_q_string(&qs(code.replace('-', "_"))));
    if code == i18n::SOURCE {
        return;
    }
    let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("translations"))) else { return };
    let lower = code.replace('-', "_").to_lowercase();
    let primary = lower.split('_').next().unwrap_or("").to_string();
    for name in [format!("qtbase_{}", code.replace('-', "_")), format!("qtbase_{lower}"), format!("qtbase_{primary}")] {
        let file = dir.join(format!("{name}.qm"));
        if file.is_file() {
            let tr = qt_core::QTranslator::new_0a();
            if tr.load_q_string(&qs(file.to_string_lossy())) {
                qt_core::QCoreApplication::install_translator(tr.into_ptr());
            }
            return;
        }
    }
}
