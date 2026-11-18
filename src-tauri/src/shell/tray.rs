use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::models::FlyoutView;

use super::flyout;

const OPEN: &str = "open";
const LOCK: &str = "lock";
const SETTINGS: &str = "settings";
const QUIT: &str = "quit";

pub fn install<R: Runtime>(app: &AppHandle<R>, lock_available: bool) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, OPEN, "Open account switcher", true, None::<&str>)?;
    let lock = MenuItem::with_id(app, LOCK, "Lock screen", lock_available, None::<&str>)?;
    let settings = MenuItem::with_id(app, SETTINGS, "Settings", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &lock, &settings, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Fluent Account Switcher")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            OPEN => flyout::show(app, FlyoutView::List),
            SETTINGS => flyout::show(app, FlyoutView::Settings),
            LOCK => lock_from_tray(app.clone()),
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
    Ok(())
}

fn lock_from_tray<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<crate::app::AppState>();
        let result = state.service.lock(&|| flyout::hide(&app));
        if let Err(error) = result {
            tracing::warn!(code = ?error.code, "lock from tray failed");
        }
    });
}
