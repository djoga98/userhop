pub mod flyout;
pub mod positioning;
pub mod shortcut;
#[cfg(any(target_os = "macos", all(test, unix)))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub mod single_instance_macos;
pub mod tray;

use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri::{Runtime, Theme, WebviewWindow};

use crate::models::Backdrop;
use crate::settings::ThemePreference;

pub fn apply_theme<R: Runtime>(window: &WebviewWindow<R>, theme: ThemePreference) {
    let native = match theme {
        ThemePreference::System => None,
        ThemePreference::Light => Some(Theme::Light),
        ThemePreference::Dark => Some(Theme::Dark),
    };
    if let Err(error) = window.set_theme(native) {
        tracing::warn!(%error, "window theme failed");
    }
}

pub fn apply_backdrop<R: Runtime>(window: &WebviewWindow<R>, backdrop: Backdrop) -> Backdrop {
    let effects = match backdrop {
        Backdrop::Acrylic => EffectsBuilder::new().effect(Effect::Acrylic).build(),
        Backdrop::Vibrancy => EffectsBuilder::new()
            .effect(Effect::Popover)
            .state(EffectState::Active)
            .radius(12.0)
            .build(),
        Backdrop::None => return solid_chrome(window),
    };
    match window.set_effects(effects) {
        Ok(()) => backdrop,
        Err(error) => {
            tracing::warn!(%error, "native backdrop unavailable");
            solid_chrome(window)
        }
    }
}

/// On Windows 10 the frameless-window shadow is drawn as a 1 px white border, so the panel's own
/// border is used instead.
fn solid_chrome<R: Runtime>(window: &WebviewWindow<R>) -> Backdrop {
    if cfg!(windows)
        && let Err(error) = window.set_shadow(false)
    {
        tracing::warn!(%error, "window shadow change failed");
    }
    Backdrop::None
}
