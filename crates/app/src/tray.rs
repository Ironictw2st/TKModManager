//! System tray icon: Show, Play (active profile), Play profile ▸, Quit. Visible when
//! "keep running in the tray" is on or the app was started with --minimized.

use crate::app::App;
use qt_core::{qs, QBox, SlotNoArgs};
use qt_gui::QIcon;
use qt_widgets::{q_system_tray_icon::ActivationReason, QMenu, QSystemTrayIcon, SlotOfActivationReason};
use std::rc::Rc;

pub struct TrayUi {
    pub icon: QBox<QSystemTrayIcon>,
    pub menu: QBox<QMenu>,
}

impl TrayUi {
    pub fn build(icon: &cpp_core::CppBox<QIcon>) -> TrayUi {
        unsafe {
            let tray = QSystemTrayIcon::new();
            tray.set_icon(icon);
            tray.set_tool_tip(&qs("TK Mod Manager"));
            let menu = QMenu::new();
            tray.set_context_menu(&menu);
            TrayUi { icon: tray, menu }
        }
    }

    pub unsafe fn connect(&self, app: &Rc<App>) {
        let this = app.clone();
        self.icon.activated().connect(&SlotOfActivationReason::new(&app.window, move |r| {
            if r == ActivationReason::Trigger || r == ActivationReason::DoubleClick {
                this.show_window();
            }
        }));
    }

    pub fn set_visible(&self, v: bool) {
        unsafe { self.icon.set_visible(v) }
    }

    pub unsafe fn rebuild(&self, app: &Rc<App>) {
        self.menu.clear();
        let st = app.st.borrow();
        let show = self.menu.add_action_q_string(&qs("Show TK Mod Manager"));
        let this = app.clone();
        show.triggered().connect(&SlotNoArgs::new(&app.window, move || this.show_window()));
        self.menu.add_separator();
        let play = self.menu.add_action_q_string(&qs(format!("Play ({})", st.profiles.active)));
        let this = app.clone();
        play.triggered().connect(&SlotNoArgs::new(&app.window, move || this.launch(None)));
        let sub = self.menu.add_menu_q_string(&qs("Play profile"));
        for p in &st.profiles.profiles {
            let a = sub.add_action_q_string(&qs(&p.name));
            let this = app.clone();
            let name = p.name.clone();
            a.triggered().connect(&SlotNoArgs::new(&app.window, move || this.launch(Some(name.clone()))));
        }
        self.menu.add_separator();
        let quit = self.menu.add_action_q_string(&qs("Quit"));
        quit.triggered().connect(&SlotNoArgs::new(&app.window, || qt_core::QCoreApplication::quit()));
    }
}
