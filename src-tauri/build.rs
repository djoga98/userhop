const COMMANDS: &[&str] = &[
    "get_accounts",
    "get_capabilities",
    "request_switch",
    "open_system_sign_in",
    "lock_session",
    "get_settings",
    "save_settings",
    "set_shortcut",
    "set_launch_at_sign_in",
    "set_shortcut_capture",
    "hide_flyout",
    "fit_flyout",
    "flyout_ready",
];

fn main() {
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    if let Err(error) = tauri_build::try_build(attributes) {
        panic!("tauri build script failed: {error:#}");
    }
}
