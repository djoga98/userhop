use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow,
};

use crate::models::{FlyoutOpened, FlyoutView};

use super::positioning::{Rect, centered_origin, flyout_origin};

pub const LABEL: &str = "flyout";
pub const EVENT_OPENED: &str = "flyout://opened";
pub const EVENT_HIDDEN: &str = "flyout://hidden";
const WIDTH: f64 = 388.0;
const MIN_HEIGHT: f64 = 120.0;
const MAX_HEIGHT: f64 = 520.0;
const MARGIN: f64 = 12.0;
const BLUR_GRACE: Duration = Duration::from_millis(250);
const REOPEN_GUARD: Duration = Duration::from_millis(300);

pub struct FlyoutState {
    height: Mutex<f64>,
    shown_at: Mutex<Option<Instant>>,
    blurred_at: Mutex<Option<Instant>>,
    pub busy: AtomicBool,
}

impl Default for FlyoutState {
    fn default() -> Self {
        Self {
            height: Mutex::new(320.0),
            shown_at: Mutex::new(None),
            blurred_at: Mutex::new(None),
            busy: AtomicBool::new(false),
        }
    }
}

fn state<R: Runtime>(app: &AppHandle<R>) -> tauri::State<'_, crate::app::AppState> {
    app.state::<crate::app::AppState>()
}

fn window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window(LABEL)
}

fn set_instant(slot: &Mutex<Option<Instant>>, value: Option<Instant>) {
    if let Ok(mut guard) = slot.lock() {
        *guard = value;
    }
}

fn elapsed_since(slot: &Mutex<Option<Instant>>) -> Option<Duration> {
    slot.lock().ok().and_then(|g| g.map(|t| t.elapsed()))
}

fn to_rect(position: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> Rect {
    Rect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    }
}

fn target_monitor<R: Runtime>(app: &AppHandle<R>) -> Option<Monitor> {
    app.cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
}

fn place<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>, monitor: Option<Monitor>) {
    let logical_height = state(app).flyout.height.lock().map_or(320.0, |h| *h);
    let Some(monitor) = monitor else {
        return;
    };
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let work = to_rect(work.position, work.size);
    let max_height = (f64::from(work.height) / scale - 2.0 * MARGIN).clamp(MIN_HEIGHT, MAX_HEIGHT);
    let height = logical_height.clamp(MIN_HEIGHT, max_height);
    let size = PhysicalSize::new(
        (WIDTH * scale).round() as u32,
        (height * scale).round() as u32,
    );
    if let Err(error) = window.set_size(size) {
        tracing::warn!(%error, "flyout resize failed");
    }
    if crate::platform::is_wayland_session() {
        return;
    }
    let monitor_rect = to_rect(*monitor.position(), *monitor.size());
    let margin = (MARGIN * scale).round() as u32;
    let (x, y) = if work.width == 0 || work.height == 0 {
        centered_origin(monitor_rect, (size.width, size.height))
    } else {
        flyout_origin(monitor_rect, work, (size.width, size.height), margin)
    };
    if let Err(error) = window.set_position(PhysicalPosition::new(x, y)) {
        tracing::warn!(%error, "flyout positioning failed");
    }
}

pub fn show<R: Runtime>(app: &AppHandle<R>, view: FlyoutView) {
    let Some(window) = window(app) else { return };
    tracing::debug!(?view, "flyout show");
    place(app, &window, target_monitor(app));
    set_instant(&state(app).flyout.shown_at, Some(Instant::now()));
    if let Err(error) = window.show().and_then(|()| window.set_focus()) {
        tracing::warn!(%error, "flyout show failed");
    }
    if let Err(error) = app.emit_to(LABEL, EVENT_OPENED, FlyoutOpened { view }) {
        tracing::warn!(%error, "flyout open event failed");
    }
}

pub fn hide<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = window(app) else { return };
    tracing::debug!("flyout hide");
    let state = state(app);
    set_instant(&state.flyout.shown_at, None);
    state.flyout.busy.store(false, Ordering::Release);
    state.capturing_shortcut.store(false, Ordering::Release);
    if let Err(error) = window.hide() {
        tracing::warn!(%error, "flyout hide failed");
    }
    let _ = app.emit_to(LABEL, EVENT_HIDDEN, ());
}

/// Re-shows the flyout without resetting the frontend, used to surface a failed handoff.
pub fn reveal<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = window(app) else { return };
    set_instant(&state(app).flyout.shown_at, Some(Instant::now()));
    if let Err(error) = window.show().and_then(|()| window.set_focus()) {
        tracing::warn!(%error, "flyout reveal failed");
    }
}

pub fn is_visible<R: Runtime>(app: &AppHandle<R>) -> bool {
    window(app).is_some_and(|w| w.is_visible().unwrap_or(false))
}

pub fn toggle<R: Runtime>(app: &AppHandle<R>, view: FlyoutView) {
    tracing::debug!(visible = is_visible(app), "flyout toggle");
    if is_visible(app) {
        hide(app);
        return;
    }
    let just_dismissed =
        elapsed_since(&state(app).flyout.blurred_at).is_some_and(|e| e < REOPEN_GUARD);
    if !just_dismissed {
        show(app, view);
    }
}

pub fn on_focus_lost<R: Runtime>(app: &AppHandle<R>) {
    let flyout = &state(app).flyout;
    if flyout.busy.load(Ordering::Acquire) || !is_visible(app) {
        return;
    }
    if elapsed_since(&flyout.shown_at).is_none_or(|e| e < BLUR_GRACE) {
        return;
    }
    set_instant(&flyout.blurred_at, Some(Instant::now()));
    tracing::debug!("flyout lost focus");
    hide(app);
}

pub fn fit<R: Runtime>(app: &AppHandle<R>, logical_height: f64) {
    if !logical_height.is_finite() {
        return;
    }
    let height = logical_height.clamp(MIN_HEIGHT, MAX_HEIGHT);
    if let Ok(mut guard) = state(app).flyout.height.lock() {
        if (*guard - height).abs() < 0.5 {
            return;
        }
        *guard = height;
    }
    if let Some(window) = window(app) {
        let monitor = window
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| target_monitor(app));
        place(app, &window, monitor);
    }
}
