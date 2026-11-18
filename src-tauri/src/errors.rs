use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    PermissionDenied,
    SessionNotFound,
    SessionNoLongerActive,
    NotLocalGraphical,
    RemoteSession,
    PolicyDisabled,
    UnsupportedPlatform,
    UnsupportedDesktop,
    UnsupportedLoginManager,
    SecureHandoffUnavailable,
    ShortcutConflict,
    ShortcutInvalid,
    ShortcutUnsupported,
    AutostartFailed,
    SettingsIo,
    OsApiFailure,
    InvalidRequest,
    Busy,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[error("{code:?}")]
pub struct SwitcherError {
    pub code: ErrorCode,
}

impl SwitcherError {
    pub const fn new(code: ErrorCode) -> Self {
        Self { code }
    }

    /// Records OS failure detail locally without user identities; only the code crosses IPC.
    pub fn os(api: &'static str, detail: impl std::fmt::Display) -> Self {
        tracing::warn!(api, %detail, "native call failed");
        Self::new(ErrorCode::OsApiFailure)
    }

    pub fn with_log(code: ErrorCode, api: &'static str, detail: impl std::fmt::Display) -> Self {
        tracing::warn!(api, ?code, %detail, "native call failed");
        Self::new(code)
    }
}

impl From<ErrorCode> for SwitcherError {
    fn from(code: ErrorCode) -> Self {
        Self::new(code)
    }
}

pub type SwitcherResult<T> = Result<T, SwitcherError>;
