use serde::{Deserialize, Serialize};

use crate::errors::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum SessionState {
    Active,
    Inactive,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum SessionKind {
    LocalGraphical,
    Remote,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub state: SessionState,
    pub kind: SessionKind,
    pub seat_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum Presence {
    Current,
    OwnSession,
    SignedIn,
    SignedInRemote,
    NotSignedIn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(tag = "mode", rename_all = "kebab-case")]
pub enum SwitchAvailability {
    Current,
    #[serde(rename_all = "camelCase")]
    Direct {
        session_id: String,
    },
    SystemSignIn,
    Unavailable {
        reason: ErrorCode,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct AccountEntry {
    pub key: String,
    pub account_id: String,
    pub session_id: Option<String>,
    pub display_name: String,
    pub user_name: String,
    pub is_current_user: bool,
    pub presence: Presence,
    pub sessions: Vec<SessionSummary>,
    pub availability: SwitchAvailability,
    pub recent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct AccountSnapshot {
    pub entries: Vec<AccountEntry>,
    pub directory_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwitchRequest {
    pub account_id: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum HandoffMode {
    Direct,
    SystemSignIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SwitchResult {
    HandoffRequested { mode: HandoffMode },
    NoOpCurrentSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Capability {
    Available,
    Unavailable { reason: ErrorCode },
}

impl Capability {
    pub const fn unavailable(reason: ErrorCode) -> Self {
        Self::Unavailable { reason }
    }

    pub const fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    Windows,
    Linux,
    Macos,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum SignInSurface {
    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    WindowsSignIn,
    #[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
    Gdm,
    #[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
    FreedesktopDisplayManager,
    #[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
    MacosLoginWindow,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum Backdrop {
    #[cfg_attr(not(windows), allow(dead_code))]
    Acrylic,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Vibrancy,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub platform: Platform,
    pub session_discovery: Capability,
    pub account_directory: Capability,
    pub system_sign_in: Capability,
    pub sign_in_surface: SignInSurface,
    pub same_user_session_switch: Capability,
    pub lock_screen: Capability,
    pub backdrop: Backdrop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum FlyoutView {
    List,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct FlyoutOpened {
    pub view: FlyoutView,
}
