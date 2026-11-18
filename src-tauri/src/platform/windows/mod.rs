mod native;
mod watcher;

use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::platform::windows_model::{
    RawWindowsPolicy, RawWindowsState, assemble_snapshot, capabilities, is_windows_11,
};
use crate::session::{ProviderCapabilities, SessionKey, SessionProvider, SystemSnapshot};

pub use watcher::spawn_session_watcher;

pub struct WindowsProvider;

impl WindowsProvider {
    fn raw_state() -> SwitcherResult<RawWindowsState> {
        let own_session_id = native::own_session_id()?;
        let sessions = native::enumerate_sessions()?;
        let current_user_name = sessions
            .iter()
            .find(|s| s.id == own_session_id)
            .and_then(|s| s.user_name.clone())
            .ok_or_else(|| SwitcherError::new(ErrorCode::OsApiFailure))?;
        Ok(RawWindowsState {
            own_session_id,
            current_user_sid: native::current_user_sid()?,
            current_user_full_name: native::current_user_full_name(&current_user_name),
            current_user_name,
            sessions,
            local_accounts: native::local_accounts(),
        })
    }

    fn policy() -> RawWindowsPolicy {
        let own_session_id = native::own_session_id().unwrap_or(u32::MAX);
        RawWindowsPolicy {
            own_session_id,
            console_session_id: native::active_console_session_id(),
            own_protocol: native::query_protocol(own_session_id),
            hide_fast_user_switching: native::hide_fast_user_switching(),
            lock_disabled: native::lock_workstation_disabled(),
            directory_available: true,
        }
    }
}

impl SessionProvider for WindowsProvider {
    fn snapshot(&self) -> SwitcherResult<SystemSnapshot> {
        let raw = Self::raw_state()?;
        assemble_snapshot(&raw).ok_or_else(|| SwitcherError::new(ErrorCode::OsApiFailure))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        capabilities(&Self::policy())
    }

    fn activate_own_session(&self, _session: &SessionKey) -> SwitcherResult<()> {
        Err(SwitcherError::new(ErrorCode::UnsupportedPlatform))
    }

    /// Locking keeps the console session attached to its displays, unlike `WTSDisconnectSession`,
    /// which tears down the display topology and rearranges desktops on multi-monitor setups.
    /// The sign-in screen then lists the other accounts and Windows performs the real switch.
    fn open_system_sign_in(&self) -> SwitcherResult<()> {
        if !capabilities(&Self::policy()).system_sign_in.is_available() {
            return Err(SwitcherError::new(ErrorCode::SecureHandoffUnavailable));
        }
        native::lock_workstation()
    }

    fn lock_current_session(&self) -> SwitcherResult<()> {
        native::lock_workstation()
    }
}

pub fn supports_acrylic() -> bool {
    is_windows_11(native::windows_build_number())
}

#[cfg(test)]
mod native_tests {
    use super::*;
    use crate::models::{Presence, SessionKind};
    use crate::session::policy::build_snapshot;

    /// Read-only check against the real OS: run with `cargo test -- --ignored native_`.
    #[test]
    #[ignore = "requires an interactive Windows session"]
    fn native_snapshot_identifies_current_console_user() {
        let provider = WindowsProvider;
        let snapshot = provider.snapshot().expect("WTS enumeration");
        let caps = provider.capabilities();
        let view = build_snapshot(&snapshot, &caps).expect("policy");
        let current = &view.entries[0];
        assert!(current.is_current_user);
        assert_eq!(current.presence, Presence::Current);
        assert!(current.account_id.starts_with("win:S-1-5-"));
        assert!(
            current.session_id.is_some(),
            "own WTS session must be identified"
        );
        assert!(
            view.entries
                .iter()
                .filter(|e| e.presence == Presence::Current)
                .count()
                == 1
        );
        let local_sessions = snapshot
            .sessions
            .iter()
            .filter(|s| s.kind == SessionKind::LocalGraphical)
            .count();
        eprintln!(
            "entries={} sessions={} local_graphical={} directory_complete={} sign_in={:?} lock={:?} acrylic={}",
            view.entries.len(),
            snapshot.sessions.len(),
            local_sessions,
            snapshot.directory_complete,
            caps.system_sign_in,
            caps.lock_screen,
            supports_acrylic(),
        );
    }
}
