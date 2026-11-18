use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::models::FlyoutView;

use super::flyout;

const OPEN: &str = "open";
const RECENT: &str = "recent";
const LOCK: &str = "lock";
const SETTINGS: &str = "settings";
const QUIT: &str = "quit";

/// Windows menus treat `&` as a mnemonic marker.
fn menu_text(label: &str) -> String {
    if cfg!(windows) {
        label.replace('&', "&&")
    } else {
        label.to_owned()
    }
}

pub fn recent_text(label: &str) -> String {
    menu_text(&format!("Switch back to {label}"))
}

pub fn install(app: &AppHandle, lock_available: bool) -> tauri::Result<MenuItem<tauri::Wry>> {
    let open = MenuItem::with_id(app, OPEN, "Open account switcher", true, None::<&str>)?;
    let recent = MenuItem::with_id(app, RECENT, "Switch back", false, None::<&str>)?;
    let lock = MenuItem::with_id(app, LOCK, "Lock screen", lock_available, None::<&str>)?;
    let settings = MenuItem::with_id(app, SETTINGS, "Settings", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &recent, &lock, &settings, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Fluent Account Switcher")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            OPEN => flyout::show(app, FlyoutView::List),
            SETTINGS => flyout::show(app, FlyoutView::Settings),
            LOCK => lock_from_tray(app.clone()),
            RECENT => crate::app::switch_back_from_tray(app),
            QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                flyout::toggle(tray.app_handle(), FlyoutView::List);
            }
        });
    if cfg!(target_os = "macos") {
        builder = builder
            .icon(tauri::include_image!("icons/tray-template.png"))
            .icon_as_template(true);
    } else if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(recent)
}

fn lock_from_tray(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<crate::app::AppState>();
        let result = state.service.lock(&|| flyout::hide(&app));
        if let Err(error) = result {
            tracing::warn!(code = ?error.code, "lock from tray failed");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_menu_text_keeps_ampersands_literal_on_windows() {
        let text = recent_text("Sam & Co");
        if cfg!(windows) {
            assert_eq!(text, "Switch back to Sam && Co");
        } else {
            assert_eq!(text, "Switch back to Sam & Co");
        }
    }
}
