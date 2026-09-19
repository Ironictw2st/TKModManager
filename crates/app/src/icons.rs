//! Small painted icons (status dots) and the colors used for status text.

use cpp_core::CppBox;
use qt_core::{GlobalColor, PenStyle, QFlags};
use qt_gui::q_painter::RenderHint;
use qt_gui::{QBrush, QColor, QIcon, QPainter, QPen, QPixmap};
use tkmm_core::status::StatusKind;

pub fn rgb(r: i32, g: i32, b: i32) -> CppBox<QColor> {
    unsafe { QColor::from_rgb_3a(r, g, b) }
}

pub fn green() -> CppBox<QColor> {
    rgb(46, 160, 67)
}
pub fn amber() -> CppBox<QColor> {
    rgb(210, 153, 34)
}
pub fn red() -> CppBox<QColor> {
    rgb(218, 54, 51)
}
pub fn grey() -> CppBox<QColor> {
    rgb(140, 140, 140)
}
pub fn blue() -> CppBox<QColor> {
    rgb(47, 129, 247)
}

pub fn status_color(kind: StatusKind) -> CppBox<QColor> {
    match kind {
        StatusKind::Pending => red(),
        StatusKind::Old => amber(),
        StatusKind::Ok => green(),
        StatusKind::Unknown | StatusKind::Local => grey(),
    }
}

/// A 12 px dot: filled for Pending/Old/Ok, hollow for Unknown/Local.
pub fn status_dot(kind: StatusKind) -> CppBox<QIcon> {
    unsafe {
        let pm = QPixmap::from_2_int(12, 12);
        pm.fill_1a(&QColor::from_global_color(GlobalColor::Transparent));
        let p = QPainter::new_1a(&pm);
        p.set_render_hint_1a(RenderHint::Antialiasing);
        let c = status_color(kind);
        match kind {
            StatusKind::Unknown | StatusKind::Local => {
                let pen = QPen::from_q_color(&c);
                pen.set_width_f(1.5);
                p.set_pen_q_pen(&pen);
                p.set_brush_global_color(GlobalColor::Transparent);
                p.draw_ellipse_4_int(2, 2, 8, 8);
            }
            _ => {
                p.set_pen_pen_style(PenStyle::NoPen);
                p.set_brush_q_brush(&QBrush::from_q_color(&c));
                p.draw_ellipse_4_int(1, 1, 10, 10);
            }
        }
        p.end();
        QIcon::from_q_pixmap(&pm)
    }
}

#[allow(dead_code)]
pub fn no_flags() -> QFlags<qt_core::AlignmentFlag> {
    QFlags::from(0)
}
