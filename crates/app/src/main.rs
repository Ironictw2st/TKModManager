#![windows_subsystem = "windows"]
//! TK Mod Manager — Qt Widgets UI over `tkmm_core`.

mod app;
mod details;
mod dialogs;
mod events;
mod icons;
mod launch_panel;
mod mod_list;
mod settings_ui;
mod single_instance;
mod state;
mod tabs;
mod theme;
mod tray;

use qt_core::qs;
use qt_widgets::QApplication;
use tkmm_core::{cli, update, AppContext};

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let startup = cli::parse(argv.clone());

    // A second copy hands its command line to the running one and exits.
    let listener = match single_instance::acquire(&argv) {
        Ok(l) => l,
        Err(()) => return,
    };
    update::cleanup_old_files();

    QApplication::init(move |_| unsafe {
        theme::apply();
        qt_gui::QGuiApplication::set_quit_on_last_window_closed(false);
        qt_core::QCoreApplication::set_application_name(&qs("TK Mod Manager"));

        let ctx = AppContext::load();
        let (bus, rx) = events::Bus::new();
        single_instance::listen(listener, bus.clone());
        let app = app::App::new(ctx, bus, rx, startup);
        app.start();
        QApplication::exec()
    })
}
