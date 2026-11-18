use std::str::FromStr;

use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::settings::{ShortcutState, ShortcutStatus};

const MAX_ACCELERATOR_LEN: usize = 64;

/// Returns a parsed shortcut and its canonical `Control+Alt+Shift+Super+Code` form.
pub fn canonicalize(input: &str) -> SwitcherResult<(Shortcut, String)> {
    let invalid = || SwitcherError::new(ErrorCode::ShortcutInvalid);
    if input.is_empty() || input.len() > MAX_ACCELERATOR_LEN || !input.is_ascii() {
        return Err(invalid());
    }
    let shortcut = Shortcut::from_str(input).map_err(|_| invalid())?;
    let required = Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER;
    if !shortcut.mods.intersects(required) || is_modifier_key(shortcut.key) {
        return Err(invalid());
    }
    let mut parts = Vec::with_capacity(5);
    for (flag, name) in [
        (Modifiers::CONTROL, "Control"),
        (Modifiers::ALT, "Alt"),
        (Modifiers::SHIFT, "Shift"),
        (Modifiers::SUPER, "Super"),
    ] {
        if shortcut.mods.contains(flag) {
            parts.push(name.to_owned());
        }
    }
    parts.push(shortcut.key.to_string());
    Ok((shortcut, parts.join("+")))
}

fn is_modifier_key(code: Code) -> bool {
    matches!(
        code,
        Code::ControlLeft
            | Code::ControlRight
            | Code::AltLeft
            | Code::AltRight
            | Code::ShiftLeft
            | Code::ShiftRight
            | Code::MetaLeft
            | Code::MetaRight
    )
}

/// X11 key grabs used by the plugin on Linux do not receive keys from native Wayland clients.
pub fn platform_supported() -> bool {
    !crate::platform::is_wayland_session()
}

pub fn apply<R: Runtime>(app: &AppHandle<R>, desired: Option<&str>) -> ShortcutStatus {
    let manager = app.global_shortcut();
    if let Err(error) = manager.unregister_all() {
        tracing::warn!(%error, "failed to clear shortcuts");
    }
    let Some(desired) = desired else {
        return ShortcutStatus {
            accelerator: None,
            state: ShortcutState::Disabled,
        };
    };
    let Ok((shortcut, canonical)) = canonicalize(desired) else {
        return ShortcutStatus {
            accelerator: Some(desired.to_owned()),
            state: ShortcutState::Invalid,
        };
    };
    if !platform_supported() {
        return ShortcutStatus {
            accelerator: Some(canonical),
            state: ShortcutState::Unsupported,
        };
    }
    let state = match manager.register(shortcut) {
        Ok(()) => ShortcutState::Registered,
        Err(error) => {
            tracing::warn!(%error, "shortcut registration refused");
            ShortcutState::Conflict
        }
    };
    ShortcutStatus {
        accelerator: Some(canonical),
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_form_is_stable_regardless_of_input_order() {
        let (_, a) = canonicalize("Super+Shift+KeyU").expect("valid");
        let (_, b) = canonicalize("shift+super+u").expect("valid");
        assert_eq!(a, "Shift+Super+KeyU");
        assert_eq!(a, b);
        let (_, c) = canonicalize("Control+Alt+F7").expect("valid");
        assert_eq!(c, "Control+Alt+F7");
    }

    #[test]
    fn rejects_shortcuts_that_would_capture_plain_typing() {
        for bad in [
            "KeyU",
            "Shift+KeyU",
            "",
            "Control+",
            "Control+ShiftLeft",
            "Super+Nope",
            "Ctrl+č",
        ] {
            assert_eq!(
                canonicalize(bad).unwrap_err().code,
                ErrorCode::ShortcutInvalid,
                "{bad:?}"
            );
        }
        assert!(canonicalize(&"Control+".repeat(20)).is_err());
    }
}
