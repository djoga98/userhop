use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::models::Capability;

pub const SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_SHORTCUT: &str = "Shift+Super+KeyU";
const MAX_FILE_BYTES: u64 = 16 * 1024;
const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub theme: ThemePreference,
    pub compact_rows: bool,
    pub reduce_motion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSettings {
    pub version: u32,
    #[serde(default)]
    pub preferences: Preferences,
    #[serde(default = "default_shortcut")]
    pub shortcut: Option<String>,
}

fn default_shortcut() -> Option<String> {
    Some(DEFAULT_SHORTCUT.to_owned())
}

impl Default for StoredSettings {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            preferences: Preferences::default(),
            shortcut: default_shortcut(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum ShortcutState {
    Registered,
    Disabled,
    Conflict,
    Invalid,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub accelerator: Option<String>,
    pub state: ShortcutState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct LaunchAtSignIn {
    pub capability: Capability,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub preferences: Preferences,
    pub shortcut: ShortcutStatus,
    pub launch_at_sign_in: LaunchAtSignIn,
    pub version: String,
}

/// Any unreadable, oversized, malformed or future-version file yields safe defaults.
pub fn parse(bytes: &[u8]) -> StoredSettings {
    match serde_json::from_slice::<StoredSettings>(bytes) {
        Ok(settings) if settings.version == SCHEMA_VERSION => sanitize(settings),
        Ok(settings) => {
            tracing::warn!(version = settings.version, "unsupported settings version");
            StoredSettings::default()
        }
        Err(error) => {
            tracing::warn!(%error, "settings file is invalid");
            StoredSettings::default()
        }
    }
}

fn sanitize(mut settings: StoredSettings) -> StoredSettings {
    if settings
        .shortcut
        .as_deref()
        .is_some_and(|s| s.is_empty() || s.len() > 64)
    {
        settings.shortcut = default_shortcut();
    }
    settings
}

pub struct SettingsStore {
    path: Option<PathBuf>,
    current: StoredSettings,
}

impl SettingsStore {
    pub fn load(dir: Option<PathBuf>) -> Self {
        let path = dir.map(|d| d.join(FILE_NAME));
        let current = path.as_deref().map(read_file).unwrap_or_default();
        Self { path, current }
    }

    pub fn get(&self) -> &StoredSettings {
        &self.current
    }

    pub fn update(&mut self, apply: impl FnOnce(&mut StoredSettings)) -> SwitcherResult<()> {
        let mut next = self.current.clone();
        apply(&mut next);
        next.version = SCHEMA_VERSION;
        let path = self
            .path
            .as_deref()
            .ok_or_else(|| SwitcherError::new(ErrorCode::SettingsIo))?;
        write_atomic(path, &next)?;
        self.current = next;
        Ok(())
    }
}

fn read_file(path: &Path) -> StoredSettings {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return StoredSettings::default();
        }
        Err(error) => {
            tracing::warn!(%error, "settings file unreadable");
            return StoredSettings::default();
        }
    };
    let mut bytes = Vec::new();
    match file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes) {
        Ok(len) if len as u64 <= MAX_FILE_BYTES => parse(&bytes),
        Ok(_) => {
            tracing::warn!("settings file too large");
            StoredSettings::default()
        }
        Err(error) => {
            tracing::warn!(%error, "settings file unreadable");
            StoredSettings::default()
        }
    }
}

fn write_atomic(path: &Path, settings: &StoredSettings) -> SwitcherResult<()> {
    let io =
        |error: std::io::Error| SwitcherError::with_log(ErrorCode::SettingsIo, "settings", error);
    let dir = path
        .parent()
        .ok_or_else(|| SwitcherError::new(ErrorCode::SettingsIo))?;
    fs::create_dir_all(dir).map_err(io)?;
    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|e| SwitcherError::with_log(ErrorCode::SettingsIo, "settings", e))?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp).map_err(io)?;
        file.write_all(&bytes).map_err(io)?;
        file.sync_all().map_err(io)?;
    }
    fs::rename(&tmp, path).map_err(io)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fas-settings-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn defaults_follow_system_and_use_default_shortcut() {
        let settings = StoredSettings::default();
        assert_eq!(settings.preferences.theme, ThemePreference::System);
        assert!(!settings.preferences.reduce_motion);
        assert_eq!(settings.shortcut.as_deref(), Some(DEFAULT_SHORTCUT));
    }

    #[test]
    fn malformed_or_future_files_restore_defaults() {
        assert_eq!(parse(b"{not json"), StoredSettings::default());
        assert_eq!(
            parse(br#"{"version":99,"shortcut":null}"#),
            StoredSettings::default()
        );
        assert_eq!(
            parse(br#"{"version":1,"preferences":{"theme":"neon"}}"#),
            StoredSettings::default()
        );
    }

    #[test]
    fn disabled_shortcut_survives_and_missing_fields_default() {
        let parsed = parse(br#"{"version":1,"shortcut":null}"#);
        assert_eq!(parsed.shortcut, None);
        let parsed = parse(br#"{"version":1,"preferences":{"theme":"dark","compactRows":true,"reduceMotion":false}}"#);
        assert_eq!(parsed.preferences.theme, ThemePreference::Dark);
        assert!(parsed.preferences.compact_rows);
        assert_eq!(parsed.shortcut.as_deref(), Some(DEFAULT_SHORTCUT));
    }

    #[test]
    fn oversized_shortcut_is_reset() {
        let json = format!(r#"{{"version":1,"shortcut":"{}"}}"#, "a".repeat(200));
        assert_eq!(
            parse(json.as_bytes()).shortcut.as_deref(),
            Some(DEFAULT_SHORTCUT)
        );
    }

    #[test]
    fn round_trips_through_disk_and_contains_only_preferences() {
        let dir = temp_dir("roundtrip");
        let mut store = SettingsStore::load(Some(dir.clone()));
        store
            .update(|s| {
                s.preferences.theme = ThemePreference::Light;
                s.shortcut = Some("Control+Alt+KeyK".into());
            })
            .expect("save");
        let raw = fs::read_to_string(dir.join(FILE_NAME)).expect("file");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("json");
        let mut keys: Vec<_> = value.as_object().expect("object").keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["preferences", "shortcut", "version"]);
        let reloaded = SettingsStore::load(Some(dir.clone()));
        assert_eq!(reloaded.get().preferences.theme, ThemePreference::Light);
        assert_eq!(reloaded.get().shortcut.as_deref(), Some("Control+Alt+KeyK"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn oversized_file_restores_defaults() {
        let dir = temp_dir("oversized");
        fs::create_dir_all(&dir).expect("dir");
        fs::write(
            dir.join(FILE_NAME),
            vec![b' '; (MAX_FILE_BYTES + 10) as usize],
        )
        .expect("write");
        assert_eq!(
            SettingsStore::load(Some(dir.clone())).get(),
            &StoredSettings::default()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_config_dir_fails_saves_without_panicking() {
        let mut store = SettingsStore::load(None);
        assert_eq!(
            store.update(|_| {}).unwrap_err().code,
            ErrorCode::SettingsIo
        );
    }
}
