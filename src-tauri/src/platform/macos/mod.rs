mod native;
mod watcher;

use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::platform::macos_model::{
    RawMacEnvironment, RawMacState, assemble_snapshot, capabilities,
};
use crate::session::{ProviderCapabilities, SessionKey, SessionProvider, SystemSnapshot};

pub use watcher::spawn_session_watcher;

pub struct MacProvider;

impl MacProvider {
    fn environment() -> RawMacEnvironment {
        let login = native::login_framework();
        RawMacEnvironment {
            own_uid: native::own_uid(),
            console_uid: native::console_uid(),
            can_switch_to_login_window: login.is_some_and(|l| l.switch_to_login_window.is_some()),
            can_lock: login.is_some_and(|l| l.lock_screen.is_some()),
            fast_user_switching: native::fast_user_switching_preference(),
        }
    }
}

impl SessionProvider for MacProvider {
    fn snapshot(&self) -> SwitcherResult<SystemSnapshot> {
        let own_uid = native::own_uid();
        let own = native::user_by_uid(own_uid)
            .ok_or_else(|| SwitcherError::new(ErrorCode::OsApiFailure))?;
        Ok(assemble_snapshot(&RawMacState {
            own_uid,
            own_name: own.name,
            own_full_name: own.full_name,
            console_uid: native::console_uid(),
            users: native::local_users(),
            logins: native::logins(),
        }))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        capabilities(&Self::environment())
    }

    fn activate_own_session(&self, _session: &SessionKey) -> SwitcherResult<()> {
        Err(SwitcherError::new(ErrorCode::UnsupportedPlatform))
    }

    fn open_system_sign_in(&self) -> SwitcherResult<()> {
        if !capabilities(&Self::environment())
            .system_sign_in
            .is_available()
        {
            return Err(SwitcherError::new(ErrorCode::SecureHandoffUnavailable));
        }
        native::switch_to_login_window()
    }

    fn lock_current_session(&self) -> SwitcherResult<()> {
        native::lock_screen()
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;
    use crate::models::Presence;
    use crate::session::policy::build_snapshot;

    /// Read-only check against the real system: run with `cargo test -- --ignored native_`.
    #[test]
    #[ignore = "requires a logged-in macOS session"]
    fn native_snapshot_identifies_current_user_by_uid() {
        let provider = MacProvider;
        let snapshot = provider.snapshot().expect("directory and utmpx");
        let caps = provider.capabilities();
        let view = build_snapshot(&snapshot, &caps).expect("policy");
        assert_eq!(view.entries[0].presence, Presence::Current);
        assert_eq!(
            view.entries[0].account_id,
            format!("mac:uid:{}", native::own_uid())
        );
        eprintln!(
            "entries={} sessions={} current_session={} sign_in={:?} lock={:?} login_framework={}",
            view.entries.len(),
            snapshot.sessions.len(),
            snapshot.current_session.is_some(),
            caps.system_sign_in,
            caps.lock_screen,
            native::login_framework().is_some(),
        );
    }
}
