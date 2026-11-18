pub mod flyout;
pub mod positioning;
pub mod shortcut;
pub mod tray;

use tauri::window::{Effect, EffectsBuilder};
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
    if backdrop != Backdrop::Acrylic {
        return Backdrop::None;
    }
    match window.set_effects(EffectsBuilder::new().effect(Effect::Acrylic).build()) {
        Ok(()) => Backdrop::Acrylic,
        Err(error) => {
            tracing::warn!(%error, "acrylic unavailable");
            Backdrop::None
        }
    }
}
