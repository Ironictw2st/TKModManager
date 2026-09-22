#![windows_subsystem = "windows"]
//! TK Mod Manager — Qt Widgets UI over `tkmm_core`.

mod app;
mod details;
mod dialogs;
mod events;
mod icons;
mod jobs;
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

/// Release builds abort on panic (no unwinding, no console), so write what happened to
/// `crash.log` before the process goes; the next start offers to show it.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let place = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".into());
        let text = format!(
            "TK Mod Manager {} crashed at {} (unix {secs})\nthread: {}\npanic: {msg}\nat: {place}\n\n{}\n",
            env!("CARGO_PKG_VERSION"),
            tkmm_core::fmt::format_date(secs),
            std::thread::current().name().unwrap_or("?"),
            std::backtrace::Backtrace::force_capture()
        );
        let path = jobs::crash_log_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, text);
    }));
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    // Helper mode for "Force update": talk to Steam, report on stdout, exit. No UI.
    if argv.get(1).map(String::as_str) == Some(tkmm_core::steam_ugc::HELPER_FLAG) {
        let ids = tkmm_core::steam_ugc::parse_ids(argv.get(2).map(String::as_str).unwrap_or(""));
        std::process::exit(tkmm_core::steam_ugc::run_helper(&ids));
    }
    install_panic_hook();
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
