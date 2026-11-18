use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::ShortcutState as KeyState;

use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::models::{
    AccountSnapshot, Backdrop, Capability, FlyoutView, PlatformCapabilities, SwitchRequest,
    SwitchResult,
};
use crate::platform::{self, SessionEvent};
use crate::session::service::SessionService;
use crate::settings::{
    DEFAULT_SHORTCUT, LaunchAtSignIn, Preferences, SettingsStore, SettingsView, ShortcutState,
    ShortcutStatus,
};
use crate::shell::{self, flyout, shortcut, tray};

pub const EVENT_SESSIONS_CHANGED: &str = "sessions://changed";
const ARG_AUTOSTART: &str = "--autostart";
const ARG_TOGGLE: &str = "--toggle";
const ARG_HIDDEN: &str = "--hidden";

pub struct AppState {
    pub service: SessionService,
    pub settings: Mutex<SettingsStore>,
    pub flyout: flyout::FlyoutState,
    pub shortcut: Mutex<ShortcutStatus>,
    pub capturing_shortcut: AtomicBool,
    pub backdrop: Mutex<Backdrop>,
}

async fn blocking<T, F>(app: AppHandle, task: F) -> SwitcherResult<T>
where
    T: Send + 'static,
    F: FnOnce(&AppHandle) -> SwitcherResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || task(&app))
        .await
        .map_err(|_| SwitcherError::new(ErrorCode::Unknown))?
}

/// Hides the flyout right before the OS takes over and re-shows it if the OS refused.
fn with_handoff<T>(
    app: &AppHandle,
    action: impl FnOnce(&dyn Fn()) -> SwitcherResult<T>,
) -> SwitcherResult<T> {
    let hidden = AtomicBool::new(false);
    let result = action(&|| {
        hidden.store(true, Ordering::Release);
        flyout::hide(app);
    });
    if result.is_err() && hidden.load(Ordering::Acquire) {
        flyout::reveal(app);
    }
    result
}

#[tauri::command]
async fn get_accounts(app: AppHandle) -> SwitcherResult<AccountSnapshot> {
    blocking(app, |app| app.state::<AppState>().service.accounts()).await
}

#[tauri::command]
async fn get_capabilities(app: AppHandle) -> SwitcherResult<PlatformCapabilities> {
    blocking(app, |app| {
        let state = app.state::<AppState>();
        let caps = state.service.capabilities();
        let backdrop = state.backdrop.lock().map_or(Backdrop::None, |b| *b);
        Ok(PlatformCapabilities {
            platform: platform::platform(),
            session_discovery: caps.session_discovery,
            account_directory: caps.account_directory,
            system_sign_in: caps.system_sign_in,
            sign_in_surface: caps.sign_in_surface,
            same_user_session_switch: caps.same_user_session_switch,
            lock_screen: caps.lock_screen,
            backdrop,
        })
    })
    .await
}

#[tauri::command]
async fn request_switch(app: AppHandle, request: SwitchRequest) -> SwitcherResult<SwitchResult> {
    blocking(app, move |app| {
        with_handoff(app, |before| {
            app.state::<AppState>()
                .service
                .request_switch(&request, before)
        })
    })
    .await
}

#[tauri::command]
async fn open_system_sign_in(app: AppHandle) -> SwitcherResult<SwitchResult> {
    blocking(app, |app| {
        with_handoff(app, |before| {
            app.state::<AppState>().service.open_system_sign_in(before)
        })
    })
    .await
}

#[tauri::command]
async fn lock_session(app: AppHandle) -> SwitcherResult<()> {
    blocking(app, |app| {
        with_handoff(app, |before| app.state::<AppState>().service.lock(before))
    })
    .await
}

fn settings_view(app: &AppHandle) -> SwitcherResult<SettingsView> {
    let state = app.state::<AppState>();
    let preferences = state
        .settings
        .lock()
        .map(|s| s.get().preferences)
        .map_err(|_| SwitcherError::new(ErrorCode::Unknown))?;
    let shortcut = state
        .shortcut
        .lock()
        .map(|s| s.clone())
        .map_err(|_| SwitcherError::new(ErrorCode::Unknown))?;
    let launch_at_sign_in = match app.autolaunch().is_enabled() {
        Ok(enabled) => LaunchAtSignIn {
            capability: Capability::Available,
            enabled,
        },
        Err(error) => {
            tracing::warn!(%error, "autostart state unavailable");
            LaunchAtSignIn {
                capability: Capability::unavailable(ErrorCode::AutostartFailed),
                enabled: false,
            }
        }
    };
    Ok(SettingsView {
        preferences,
        shortcut,
        launch_at_sign_in,
        default_shortcut: DEFAULT_SHORTCUT.to_owned(),
        version: app.package_info().version.to_string(),
    })
}

#[tauri::command]
async fn get_settings(app: AppHandle) -> SwitcherResult<SettingsView> {
    blocking(app, settings_view).await
}

#[tauri::command]
async fn save_settings(app: AppHandle, preferences: Preferences) -> SwitcherResult<SettingsView> {
    blocking(app, move |app| {
        let state = app.state::<AppState>();
        state
            .settings
            .lock()
            .map_err(|_| SwitcherError::new(ErrorCode::Unknown))?
            .update(|s| s.preferences = preferences)?;
        if let Some(window) = app.get_webview_window(flyout::LABEL) {
            shell::apply_theme(&window, preferences.theme);
        }
        settings_view(app)
    })
    .await
}

#[tauri::command]
async fn set_shortcut(app: AppHandle, accelerator: Option<String>) -> SwitcherResult<SettingsView> {
    blocking(app, move |app| {
        let state = app.state::<AppState>();
        let canonical = accelerator
            .as_deref()
            .map(shortcut::canonicalize)
            .transpose()?
            .map(|(_, canonical)| canonical);
        let previous = state
            .settings
            .lock()
            .map(|s| s.get().shortcut.clone())
            .map_err(|_| SwitcherError::new(ErrorCode::Unknown))?;
        let status = shortcut::apply(app, canonical.as_deref());
        if status.state == ShortcutState::Conflict {
            let restored = shortcut::apply(app, previous.as_deref());
            if let Ok(mut slot) = state.shortcut.lock() {
                *slot = restored;
            }
            return Err(SwitcherError::new(ErrorCode::ShortcutConflict));
        }
        state
            .settings
            .lock()
            .map_err(|_| SwitcherError::new(ErrorCode::Unknown))?
            .update(|s| s.shortcut = canonical.clone())?;
        if let Ok(mut slot) = state.shortcut.lock() {
            *slot = status;
        }
        settings_view(app)
    })
    .await
}

#[tauri::command]
async fn set_launch_at_sign_in(app: AppHandle, enabled: bool) -> SwitcherResult<SettingsView> {
    blocking(app, move |app| {
        let autolaunch = app.autolaunch();
        let result = if enabled {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        result.map_err(|e| SwitcherError::with_log(ErrorCode::AutostartFailed, "autostart", e))?;
        settings_view(app)
    })
    .await
}

#[tauri::command]
fn set_shortcut_capture(app: AppHandle, active: bool) {
    let state = app.state::<AppState>();
    state.capturing_shortcut.store(active, Ordering::Release);
    state.flyout.busy.store(active, Ordering::Release);
}

#[tauri::command]
fn hide_flyout(app: AppHandle) {
    flyout::hide(&app);
}

#[tauri::command]
fn fit_flyout(app: AppHandle, height: f64) {
    flyout::fit(&app, height);
}

fn handle_args(app: &AppHandle, args: &[String]) {
    if args.iter().any(|a| a == ARG_TOGGLE) {
        flyout::toggle(app, FlyoutView::List);
    } else {
        flyout::show(app, FlyoutView::List);
    }
}

fn on_session_event(app: &AppHandle, event: SessionEvent) {
    match event {
        SessionEvent::CurrentLocked => {
            let app = app.clone();
            let _ = app.clone().run_on_main_thread(move || flyout::hide(&app));
        }
        SessionEvent::Changed | SessionEvent::Resumed => {
            let _ = app.emit_to(flyout::LABEL, EVENT_SESSIONS_CHANGED, ());
        }
    }
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    let handle = app.handle().clone();
    #[cfg(target_os = "macos")]
    shell::single_instance_macos::listen(&handle, handle_args);
    let settings = SettingsStore::load(handle.path().app_config_dir().ok());
    let preferences = settings.get().preferences;
    let stored_shortcut = settings.get().shortcut.clone();

    app.manage(AppState {
        service: SessionService::new(platform::create_provider()),
        settings: Mutex::new(settings),
        flyout: flyout::FlyoutState::default(),
        shortcut: Mutex::new(ShortcutStatus {
            accelerator: None,
            state: ShortcutState::Disabled,
        }),
        capturing_shortcut: AtomicBool::new(false),
        backdrop: Mutex::new(Backdrop::None),
    });

    if let Some(window) = handle.get_webview_window(flyout::LABEL) {
        shell::apply_theme(&window, preferences.theme);
        let backdrop = shell::apply_backdrop(&window, platform::native_backdrop_supported());
        if let Ok(mut slot) = handle.state::<AppState>().backdrop.lock() {
            *slot = backdrop;
        }
    }

    let status = shortcut::apply(&handle, stored_shortcut.as_deref());
    if let Ok(mut slot) = handle.state::<AppState>().shortcut.lock() {
        *slot = status;
    }

    let lock_available = handle
        .state::<AppState>()
        .service
        .capabilities()
        .lock_screen
        .is_available();
    tray::install(&handle, lock_available)?;

    let sink_handle = handle.clone();
    platform::spawn_session_watcher(Arc::new(move |event| on_session_event(&sink_handle, event)));

    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|a| a == ARG_AUTOSTART || a == ARG_HIDDEN) {
        flyout::show(&handle, FlyoutView::List);
    }
    Ok(())
}

pub fn run() {
    #[cfg(target_os = "macos")]
    if shell::single_instance_macos::forward_to_running_instance() {
        return;
    }
    let builder = tauri::Builder::default();
    #[cfg(not(target_os = "macos"))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
        handle_args(app, &args);
    }));
    builder
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    let state = app.state::<AppState>();
                    if event.state == KeyState::Pressed
                        && !state.capturing_shortcut.load(Ordering::Acquire)
                    {
                        flyout::toggle(app, FlyoutView::List);
                    }
                })
                .build(),
        )
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg(ARG_AUTOSTART)
                .build(),
        )
        .setup(setup)
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(false) => flyout::on_focus_lost(window.app_handle()),
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                flyout::hide(window.app_handle());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_accounts,
            get_capabilities,
            request_switch,
            open_system_sign_in,
            lock_session,
            get_settings,
            save_settings,
            set_shortcut,
            set_launch_at_sign_in,
            set_shortcut_capture,
            hide_flyout,
            fit_flyout,
        ])
        .build(tauri::generate_context!())
        .map(|app| {
            app.run(|_, event| {
                if let RunEvent::ExitRequested {
                    code: None, api, ..
                } = event
                {
                    api.prevent_exit();
                }
            });
        })
        .unwrap_or_else(|error| {
            tracing::error!(%error, "application failed to start");
            std::process::exit(1);
        });
}
