//! System tray: Show, Play (active profile), Play ▸ <profile>, Quit. The icon is only visible
//! when "minimize to tray" is on (or the app was started with --minimized). Menu clicks are
//! forwarded to the frontend as `tray-play` events so launching goes through the same path.

use crate::json_store;
use crate::profiles::ProfilesDoc;
use crate::state;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

const TRAY_ID: &str = "main";

fn make_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let doc: ProfilesDoc = json_store::load(&state::profiles_path()).unwrap_or_default();
    let show = MenuItem::with_id(app, "show", "Show TK Mod Manager", true, None::<&str>)?;
    let play = MenuItem::with_id(app, "play", format!("Play  ({})", doc.active), true, None::<&str>)?;
    let sub = Submenu::with_id(app, "profiles", "Play profile", true)?;
    for p in &doc.profiles {
        sub.append(&MenuItem::with_id(app, format!("play:{}", p.name), &p.name, true, None::<&str>)?)?;
    }
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[&show, &PredefinedMenuItem::separator(app)?, &play, &sub, &PredefinedMenuItem::separator(app)?, &quit],
    )
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn build(app: &AppHandle, visible: bool) -> tauri::Result<()> {
    let menu = make_menu(app)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("TK Mod Manager")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            "play" => {
                let _ = app.emit("tray-play", Option::<String>::None);
            }
            id => {
                if let Some(name) = id.strip_prefix("play:") {
                    let _ = app.emit("tray-play", Some(name.to_string()));
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let tray = builder.build(app)?;
    tray.set_visible(visible)?;
    Ok(())
}

/// Rebuild the menu after profiles changed.
pub fn refresh(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(menu) = make_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

pub fn set_visible(app: &AppHandle, visible: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_visible(visible);
    }
}
