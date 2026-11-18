use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
/// Upper bound for any view; the frontend applies the tighter per-view caps and the work area applies last.
const MAX_HEIGHT: f64 = 720.0;
const MARGIN: f64 = 12.0;
const BLUR_GRACE: Duration = Duration::from_millis(250);
const REOPEN_GUARD: Duration = Duration::from_millis(300);
const REVEAL_FALLBACK: Duration = Duration::from_millis(1500);

pub struct FlyoutState {
    height: Mutex<f64>,
    shown_at: Mutex<Option<Instant>>,
    blurred_at: Mutex<Option<Instant>>,
    pending_reveal: Mutex<Option<u64>>,
    generation: AtomicU64,
    pub busy: AtomicBool,
}

impl Default for FlyoutState {
    fn default() -> Self {
        Self {
            height: Mutex::new(320.0),
            shown_at: Mutex::new(None),
            blurred_at: Mutex::new(None),
            pending_reveal: Mutex::new(None),
            generation: AtomicU64::new(0),
            busy: AtomicBool::new(false),
        }
    }
}

fn take_pending(flyout: &FlyoutState, generation: Option<u64>) -> bool {
    let Ok(mut pending) = flyout.pending_reveal.lock() else {
        return false;
    };
    match (*pending, generation) {
        (Some(current), Some(expected)) if current != expected => false,
        (Some(_), _) => {
            *pending = None;
            true
        }
        (None, _) => false,
    }
}

fn is_pending(flyout: &FlyoutState) -> bool {
    flyout.pending_reveal.lock().is_ok_and(|p| p.is_some())
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

fn emit_opened<R: Runtime>(app: &AppHandle<R>, view: FlyoutView) {
    if let Err(error) = app.emit_to(LABEL, EVENT_OPENED, FlyoutOpened { view }) {
        tracing::warn!(%error, "flyout open event failed");
    }
}

/// The window stays hidden until the frontend has rendered the new state and reported its height
/// (`ready`), so no stale frame or wrong size is ever shown. A timer reveals it if that never comes.
pub fn show<R: Runtime>(app: &AppHandle<R>, view: FlyoutView) {
    let Some(window) = window(app) else { return };
    tracing::debug!(?view, "flyout show");
    if is_visible(app) {
        emit_opened(app, view);
        let _ = window.set_focus();
        return;
    }
    place(app, &window, target_monitor(app));
    let flyout = &state(app).flyout;
    let generation = flyout.generation.fetch_add(1, Ordering::AcqRel) + 1;
    if let Ok(mut pending) = flyout.pending_reveal.lock() {
        *pending = Some(generation);
    }
    emit_opened(app, view);
    let handle = app.clone();
    let spawned = std::thread::Builder::new()
        .name("flyout-reveal".into())
        .spawn(move || {
            std::thread::sleep(REVEAL_FALLBACK);
            let app = handle.clone();
            let _ = handle.run_on_main_thread(move || reveal_pending(&app, Some(generation)));
        });
    if spawned.is_err() {
        reveal_pending(app, Some(generation));
    }
}

fn reveal_pending<R: Runtime>(app: &AppHandle<R>, generation: Option<u64>) {
    if !take_pending(&state(app).flyout, generation) {
        return;
    }
    reveal(app);
}

/// Called by the frontend once the opened state is rendered.
pub fn ready<R: Runtime>(app: &AppHandle<R>, logical_height: f64) {
    fit(app, logical_height);
    reveal_pending(app, None);
}

pub fn hide<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = window(app) else { return };
    tracing::debug!("flyout hide");
    let state = state(app);
    take_pending(&state.flyout, None);
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
    if is_visible(app) || is_pending(&state(app).flyout) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(flyout: &FlyoutState, generation: Option<u64>) {
        *flyout.pending_reveal.lock().expect("lock") = generation;
    }

    #[test]
    fn stale_fallback_timer_cannot_reveal_a_newer_open() {
        let flyout = FlyoutState::default();
        pending(&flyout, Some(2));
        assert!(!take_pending(&flyout, Some(1)));
        assert!(is_pending(&flyout));
        assert!(take_pending(&flyout, Some(2)));
        assert!(!is_pending(&flyout));
    }

    #[test]
    fn frontend_ready_reveals_once_and_hide_cancels() {
        let flyout = FlyoutState::default();
        pending(&flyout, Some(5));
        assert!(take_pending(&flyout, None));
        assert!(!take_pending(&flyout, None));
        assert!(!take_pending(&flyout, Some(5)));
        pending(&flyout, Some(6));
        take_pending(&flyout, None);
        assert!(
            !take_pending(&flyout, Some(6)),
            "hidden before the fallback fired"
        );
    }
}
