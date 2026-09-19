//! Style and palette. Windows 11 style where its icon font exists; otherwise Fusion with a
//! palette that follows the system light/dark setting. Backgrounds stay neutral; the Windows
//! accent colour is used only for selection and focus (Qt's own dark Windows palette puts the
//! accent into AlternateBase and check-box fills, which looked like errors with a red accent).

use qt_core::{qs, ColorScheme};
use qt_gui::q_palette::{ColorGroup, ColorRole};
use qt_gui::{QColor, QFontDatabase, QGuiApplication, QPalette};
use qt_widgets::QApplication;

fn c(r: i32, g: i32, b: i32) -> cpp_core::CppBox<QColor> {
    unsafe { QColor::from_rgb_3a(r, g, b) }
}

pub unsafe fn apply() {
    let families = QFontDatabase::families_0a();
    let has_fluent = (0..families.size()).any(|i| families.at(i).to_std_string() == "Segoe Fluent Icons");
    if has_fluent && !QApplication::set_style_q_string(&qs("windows11")).is_null() {
        return; // the native style manages its own palette
    }
    QApplication::set_style_q_string(&qs("Fusion"));

    let dark = QGuiApplication::style_hints().color_scheme() == ColorScheme::Dark;
    let system = QGuiApplication::palette();
    let accent = QColor::new_copy(system.color_1a(ColorRole::Highlight));
    let p = QPalette::new();
    let set = |role: ColorRole, col: &cpp_core::CppBox<QColor>| p.set_color_2a(role, col);
    if dark {
        set(ColorRole::Window, &c(40, 40, 42));
        set(ColorRole::WindowText, &c(230, 230, 232));
        set(ColorRole::Base, &c(28, 28, 30));
        set(ColorRole::AlternateBase, &c(36, 36, 39));
        set(ColorRole::ToolTipBase, &c(50, 50, 54));
        set(ColorRole::ToolTipText, &c(230, 230, 232));
        set(ColorRole::PlaceholderText, &c(140, 140, 146));
        set(ColorRole::Text, &c(230, 230, 232));
        set(ColorRole::Button, &c(52, 52, 56));
        set(ColorRole::ButtonText, &c(230, 230, 232));
        set(ColorRole::BrightText, &c(255, 90, 90));
        set(ColorRole::Light, &c(70, 70, 75));
        set(ColorRole::Midlight, &c(58, 58, 62));
        set(ColorRole::Mid, &c(45, 45, 48));
        set(ColorRole::Dark, &c(22, 22, 24));
        set(ColorRole::Shadow, &c(10, 10, 10));
        set(ColorRole::Link, &c(96, 165, 250));
        set(ColorRole::LinkVisited, &c(167, 139, 250));
        for (role, col) in [(ColorRole::WindowText, c(120, 120, 124)), (ColorRole::Text, c(120, 120, 124)), (ColorRole::ButtonText, c(120, 120, 124))] {
            p.set_color_3a(ColorGroup::Disabled, role, &col);
        }
    } else {
        let std = QApplication::style().standard_palette();
        p.copy_from(&std);
        set(ColorRole::AlternateBase, &c(246, 246, 248));
    }
    set(ColorRole::Highlight, &accent);
    set(ColorRole::Accent, &accent);
    set(ColorRole::HighlightedText, &c(255, 255, 255));
    QApplication::set_palette_1a(&p);
}
