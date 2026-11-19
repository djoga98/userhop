pub mod ids;
pub mod policy;
pub mod service;

#[cfg(test)]
pub mod fake;

use crate::errors::SwitcherResult;
use crate::models::{Capability, SessionKind, SessionState, SignInSurface};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRecord {
    pub key: AccountKey,
    pub user_name: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub key: SessionKey,
    pub account: AccountKey,
    pub state: SessionState,
    pub kind: SessionKind,
    pub seat: Option<String>,
}

/// Facts reported by the OS for one point in time; never cached across switch requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSnapshot {
    pub current_account: AccountKey,
    pub current_session: Option<SessionKey>,
    pub accounts: Vec<AccountRecord>,
    pub sessions: Vec<SessionRecord>,
    pub directory_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub session_discovery: Capability,
    pub account_directory: Capability,
    pub system_sign_in: Capability,
    pub sign_in_surface: SignInSurface,
    pub same_user_session_switch: Capability,
    pub lock_screen: Capability,
}

pub trait SessionProvider: Send + Sync {
    fn snapshot(&self) -> SwitcherResult<SystemSnapshot>;

    fn capabilities(&self) -> ProviderCapabilities;

    /// Only called after policy confirmed the target belongs to the current OS user.
    fn activate_own_session(&self, session: &SessionKey) -> SwitcherResult<()>;

    /// Called only for an account validated against the current OS snapshot.
    fn open_system_sign_in_for(&self, _account: &AccountKey) -> SwitcherResult<()> {
        self.open_system_sign_in()
    }

    fn open_system_sign_in(&self) -> SwitcherResult<()>;

    fn lock_current_session(&self) -> SwitcherResult<()>;
}
