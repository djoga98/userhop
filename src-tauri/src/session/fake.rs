use std::sync::Mutex;

use crate::errors::{SwitcherError, SwitcherResult};
use crate::models::{Capability, SessionKind, SessionState, SignInSurface};

use super::{
    AccountKey, AccountRecord, ProviderCapabilities, SessionKey, SessionProvider, SessionRecord,
    SystemSnapshot,
};

pub fn account(key: &str, user: &str, display: Option<&str>) -> AccountRecord {
    AccountRecord {
        key: AccountKey(key.into()),
        user_name: user.into(),
        display_name: display.map(str::to_owned),
    }
}

pub fn session(key: &str, account: &str, kind: SessionKind, seat: Option<&str>) -> SessionRecord {
    SessionRecord {
        key: SessionKey(key.into()),
        account: AccountKey(account.into()),
        state: SessionState::Active,
        kind,
        seat: seat.map(str::to_owned),
    }
}

pub fn caps_all() -> ProviderCapabilities {
    ProviderCapabilities {
        session_discovery: Capability::Available,
        account_directory: Capability::Available,
        system_sign_in: Capability::Available,
        sign_in_surface: SignInSurface::Gdm,
        same_user_session_switch: Capability::Available,
        lock_screen: Capability::Available,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    ActivateOwn(SessionKey),
    SignInFor(AccountKey),
    SignIn,
    Lock,
}

pub struct FakeProvider {
    pub snapshot: Mutex<SwitcherResult<SystemSnapshot>>,
    pub capabilities: ProviderCapabilities,
    pub calls: Mutex<Vec<Call>>,
    pub fail_actions: Option<SwitcherError>,
}

impl FakeProvider {
    pub fn new(snapshot: SystemSnapshot) -> Self {
        Self {
            snapshot: Mutex::new(Ok(snapshot)),
            capabilities: caps_all(),
            calls: Mutex::new(Vec::new()),
            fail_actions: None,
        }
    }

    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().expect("calls lock").clone()
    }

    fn record(&self, call: Call) -> SwitcherResult<()> {
        self.calls.lock().expect("calls lock").push(call);
        self.fail_actions.map_or(Ok(()), Err)
    }
}

impl SessionProvider for FakeProvider {
    fn snapshot(&self) -> SwitcherResult<SystemSnapshot> {
        self.snapshot.lock().expect("snapshot lock").clone()
    }

    fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities
    }

    fn activate_own_session(&self, session: &SessionKey) -> SwitcherResult<()> {
        self.record(Call::ActivateOwn(session.clone()))
    }

    fn open_system_sign_in(&self) -> SwitcherResult<()> {
        self.record(Call::SignIn)
    }

    fn open_system_sign_in_for(&self, account: &AccountKey) -> SwitcherResult<()> {
        self.record(Call::SignInFor(account.clone()))
    }

    fn lock_current_session(&self) -> SwitcherResult<()> {
        self.record(Call::Lock)
    }
}
