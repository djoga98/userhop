mod proxies;
mod watcher;

use std::time::{Duration, Instant};

use zbus::blocking::Connection;
use zbus::blocking::fdo::DBusProxy;
use zbus::names::BusName;
use zbus::zvariant::OwnedObjectPath;

use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::platform::linux_model::{
    DisplayManager, RawLinuxAccount, RawLinuxEnvironment, RawLinuxState, RawLogindSession,
    assemble_snapshot, capabilities, is_valid_session_id, session_id_from_key,
};
use crate::session::{ProviderCapabilities, SessionKey, SessionProvider, SystemSnapshot};

use proxies::{
    AccountsProxy, AccountsUserProxy, DisplayManagerProxy, DisplayManagerSeatProxy,
    GdmLocalDisplayFactoryProxy, ManagerProxy, SeatProxy, SessionProxy, UserProxy,
};

pub use watcher::spawn_session_watcher;

const MAX_SESSIONS: usize = 256;
const MAX_ACCOUNTS: usize = 512;
const LOCK_CONFIRM_TIMEOUT: Duration = Duration::from_millis(2500);
const GDM_NAME: &str = "org.gnome.DisplayManager";
const FREEDESKTOP_DM_NAME: &str = "org.freedesktop.DisplayManager";
const FREEDESKTOP_DM_PREFIX: &str = "/org/freedesktop/DisplayManager/";

fn dbus_error(api: &'static str, error: &zbus::Error) -> SwitcherError {
    let denied = matches!(
        error,
        zbus::Error::MethodError(name, _, _)
            if name.as_str() == "org.freedesktop.DBus.Error.AccessDenied"
                || name.as_str() == "org.freedesktop.DBus.Error.InteractiveAuthorizationRequired"
    );
    if denied {
        SwitcherError::with_log(ErrorCode::PermissionDenied, api, error)
    } else {
        SwitcherError::os(api, error)
    }
}

pub fn own_uid() -> u32 {
    rustix::process::getuid().as_raw()
}

pub struct LinuxProvider {
    system: Option<Connection>,
}

impl LinuxProvider {
    pub fn new() -> Self {
        let system = Connection::system()
            .inspect_err(|error| tracing::warn!(%error, "system bus unavailable"))
            .ok();
        Self { system }
    }

    fn bus(&self) -> SwitcherResult<&Connection> {
        self.system
            .as_ref()
            .ok_or_else(|| SwitcherError::new(ErrorCode::UnsupportedDesktop))
    }

    fn manager(&self) -> SwitcherResult<ManagerProxy<'_>> {
        ManagerProxy::new(self.bus()?).map_err(|e| dbus_error("login1.Manager", &e))
    }

    fn session_proxy(&self, path: OwnedObjectPath) -> SwitcherResult<SessionProxy<'_>> {
        SessionProxy::builder(self.bus()?)
            .path(path)
            .and_then(|b| b.build())
            .map_err(|e| dbus_error("login1.Session", &e))
    }

    fn read_session(&self, id: &str, path: OwnedObjectPath) -> Option<RawLogindSession> {
        let session = self.session_proxy(path).ok()?;
        let (uid, _) = session.user().ok()?;
        Some(RawLogindSession {
            id: id.to_owned(),
            uid,
            user_name: session.name().unwrap_or_default(),
            seat: session
                .seat()
                .ok()
                .map(|(seat, _)| seat)
                .filter(|seat| !seat.is_empty()),
            class: session.class().ok()?,
            session_type: session.session_type().unwrap_or_default(),
            remote: session.remote().unwrap_or(true),
            state: session.state().unwrap_or_default(),
        })
    }

    fn list_sessions(&self) -> SwitcherResult<Vec<RawLogindSession>> {
        let listing = self
            .manager()?
            .list_sessions()
            .map_err(|e| dbus_error("ListSessions", &e))?;
        Ok(listing
            .into_iter()
            .take(MAX_SESSIONS)
            .filter(|(id, ..)| is_valid_session_id(id))
            .filter_map(|(id, _, _, _, path)| self.read_session(&id, path))
            .collect())
    }

    /// Prefers the session this process was started in, then the user's primary display session.
    fn own_session_path(&self) -> Option<(String, OwnedObjectPath)> {
        let manager = self.manager().ok()?;
        let uid = own_uid();
        let candidates = [
            std::env::var("XDG_SESSION_ID")
                .ok()
                .filter(|id| is_valid_session_id(id))
                .and_then(|id| manager.get_session(&id).ok()),
            manager.get_session_by_pid(std::process::id()).ok(),
            manager
                .get_user(uid)
                .ok()
                .and_then(|path| {
                    UserProxy::builder(self.bus().ok()?)
                        .path(path)
                        .ok()?
                        .build()
                        .ok()
                })
                .and_then(|user| user.display().ok())
                .map(|(_, path)| path)
                .filter(|path| path.as_str() != "/"),
        ];
        candidates.into_iter().flatten().find_map(|path| {
            let session = self.session_proxy(path.clone()).ok()?;
            let (session_uid, _) = session.user().ok()?;
            let id = session.id().ok()?;
            (session_uid == uid && is_valid_session_id(&id)).then_some((id, path))
        })
    }

    fn accounts_service_users(&self) -> Option<Vec<RawLinuxAccount>> {
        let bus = self.bus().ok()?;
        if !self.name_has_owner("org.freedesktop.Accounts") {
            return None;
        }
        let paths = AccountsProxy::new(bus)
            .and_then(|proxy| proxy.list_cached_users())
            .inspect_err(|error| tracing::warn!(%error, "AccountsService unavailable"))
            .ok()?;
        Some(
            paths
                .into_iter()
                .take(MAX_ACCOUNTS)
                .filter_map(|path| {
                    let user = AccountsUserProxy::builder(bus)
                        .path(path)
                        .ok()?
                        .build()
                        .ok()?;
                    Some(RawLinuxAccount {
                        uid: u32::try_from(user.uid().ok()?).ok()?,
                        user_name: user.user_name().ok()?,
                        real_name: user.real_name().ok().filter(|n| !n.trim().is_empty()),
                        system_account: user.system_account().unwrap_or(true),
                        locked: user.locked().unwrap_or(true),
                    })
                })
                .collect(),
        )
    }

    fn name_has_owner(&self, name: &str) -> bool {
        let Ok(bus) = self.bus() else { return false };
        let Ok(name) = BusName::try_from(name) else {
            return false;
        };
        DBusProxy::new(bus)
            .and_then(|proxy| proxy.name_has_owner(name).map_err(zbus::Error::from))
            .unwrap_or(false)
    }

    fn display_manager(&self) -> DisplayManager {
        if self.name_has_owner(GDM_NAME) {
            DisplayManager::Gdm
        } else if self.name_has_owner(FREEDESKTOP_DM_NAME) && self.freedesktop_seat().is_some() {
            DisplayManager::Freedesktop
        } else {
            DisplayManager::None
        }
    }

    fn freedesktop_seat(&self) -> Option<OwnedObjectPath> {
        let from_env = std::env::var("XDG_SEAT_PATH")
            .ok()
            .filter(|p| p.starts_with(FREEDESKTOP_DM_PREFIX))
            .and_then(|p| OwnedObjectPath::try_from(p).ok());
        if from_env.is_some() {
            return from_env;
        }
        let seats = DisplayManagerProxy::new(self.bus().ok()?)
            .ok()?
            .seats()
            .ok()?;
        match seats.as_slice() {
            [only] => Some(only.clone()),
            _ => None,
        }
    }

    fn environment(&self) -> RawLinuxEnvironment {
        let logind_available = self.system.is_some()
            && self.manager().is_ok()
            && self.name_has_owner("org.freedesktop.login1");
        let own_session = self
            .own_session_path()
            .and_then(|(id, path)| self.read_session(&id, path));
        let seat_can_multi_session = own_session
            .as_ref()
            .and_then(|s| s.seat.as_deref())
            .and_then(|seat| {
                let path =
                    OwnedObjectPath::try_from(format!("/org/freedesktop/login1/seat/{seat}"))
                        .ok()?;
                SeatProxy::builder(self.bus().ok()?)
                    .path(path)
                    .ok()?
                    .build()
                    .ok()
            })
            .and_then(|seat| seat.can_multi_session().ok())
            .unwrap_or(false);
        RawLinuxEnvironment {
            logind_available,
            own_session,
            seat_can_multi_session,
            display_manager: self.display_manager(),
            accounts_service: self.name_has_owner("org.freedesktop.Accounts"),
        }
    }

    /// Locks the own session and waits for logind's LockedHint so no unlocked desktop is left behind.
    fn lock_own_and_confirm(&self) -> SwitcherResult<()> {
        let (_, path) = self
            .own_session_path()
            .ok_or_else(|| SwitcherError::new(ErrorCode::SessionNotFound))?;
        let session = self.session_proxy(path)?;
        session.lock().map_err(|e| dbus_error("Session.Lock", &e))?;
        let deadline = Instant::now() + LOCK_CONFIRM_TIMEOUT;
        while Instant::now() < deadline {
            if session.locked_hint().unwrap_or(false) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        tracing::warn!("own session lock was not confirmed");
        Err(SwitcherError::new(ErrorCode::SecureHandoffUnavailable))
    }

    fn open_gdm_greeter(&self, own: &RawLogindSession) -> SwitcherResult<()> {
        let seat = own
            .seat
            .as_deref()
            .ok_or_else(|| SwitcherError::new(ErrorCode::NotLocalGraphical))?;
        let greeter = self.list_sessions()?.into_iter().find(|s| {
            s.class == "greeter" && s.seat.as_deref() == Some(seat) && s.state != "closing"
        });
        if let Some(greeter) = greeter {
            match self.manager()?.activate_session_on_seat(&greeter.id, seat) {
                Ok(()) => return Ok(()),
                Err(error) => tracing::warn!(%error, "greeter activation failed"),
            }
        }
        if seat != "seat0" {
            return Err(SwitcherError::new(ErrorCode::UnsupportedLoginManager));
        }
        GdmLocalDisplayFactoryProxy::new(self.bus()?)
            .and_then(|proxy| proxy.create_transient_display())
            .map(|_| ())
            .map_err(|e| dbus_error("CreateTransientDisplay", &e))
    }

    fn open_freedesktop_greeter(&self) -> SwitcherResult<()> {
        let path = self
            .freedesktop_seat()
            .ok_or_else(|| SwitcherError::new(ErrorCode::UnsupportedLoginManager))?;
        DisplayManagerSeatProxy::builder(self.bus()?)
            .path(path)
            .and_then(|b| b.build())
            .and_then(|seat| seat.switch_to_greeter())
            .map_err(|e| dbus_error("SwitchToGreeter", &e))
    }
}

impl SessionProvider for LinuxProvider {
    fn snapshot(&self) -> SwitcherResult<SystemSnapshot> {
        let uid = own_uid();
        let sessions = self.list_sessions()?;
        let own_session_id = self.own_session_path().map(|(id, _)| id);
        let own_user_name = self
            .manager()
            .ok()
            .and_then(|m| m.get_user(uid).ok())
            .and_then(|path| {
                UserProxy::builder(self.bus().ok()?)
                    .path(path)
                    .ok()?
                    .build()
                    .ok()
            })
            .and_then(|user| user.name().ok());
        Ok(assemble_snapshot(&RawLinuxState {
            own_uid: uid,
            own_user_name,
            own_session_id,
            sessions,
            accounts: self.accounts_service_users(),
        }))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        capabilities(&self.environment())
    }

    fn activate_own_session(&self, session: &SessionKey) -> SwitcherResult<()> {
        let id = session_id_from_key(session)
            .ok_or_else(|| SwitcherError::new(ErrorCode::InvalidRequest))?;
        let path = self
            .manager()?
            .get_session(id)
            .map_err(|_| SwitcherError::new(ErrorCode::SessionNoLongerActive))?;
        let target = self.session_proxy(path)?;
        let (uid, _) = target.user().map_err(|e| dbus_error("Session.User", &e))?;
        if uid != own_uid() {
            tracing::error!("refused cross-user direct activation");
            return Err(SwitcherError::new(ErrorCode::SecureHandoffUnavailable));
        }
        target
            .activate()
            .map_err(|e| dbus_error("Session.Activate", &e))
    }

    fn open_system_sign_in(&self) -> SwitcherResult<()> {
        let env = self.environment();
        if !capabilities(&env).system_sign_in.is_available() {
            return Err(SwitcherError::new(ErrorCode::SecureHandoffUnavailable));
        }
        let own = env
            .own_session
            .ok_or_else(|| SwitcherError::new(ErrorCode::SessionNotFound))?;
        self.lock_own_and_confirm()?;
        match env.display_manager {
            DisplayManager::Gdm => self.open_gdm_greeter(&own),
            DisplayManager::Freedesktop => self.open_freedesktop_greeter(),
            DisplayManager::None => Err(SwitcherError::new(ErrorCode::UnsupportedLoginManager)),
        }
    }

    fn lock_current_session(&self) -> SwitcherResult<()> {
        let (_, path) = self
            .own_session_path()
            .ok_or_else(|| SwitcherError::new(ErrorCode::SessionNotFound))?;
        self.session_proxy(path)?
            .lock()
            .map_err(|e| dbus_error("Session.Lock", &e))
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;
    use crate::models::Presence;
    use crate::session::policy::build_snapshot;

    /// Read-only check against the real system bus: run with `cargo test -- --ignored native_`.
    #[test]
    #[ignore = "requires systemd-logind on the system bus"]
    fn native_snapshot_identifies_current_user_by_uid() {
        let provider = LinuxProvider::new();
        let snapshot = provider.snapshot().expect("logind enumeration");
        let caps = provider.capabilities();
        let view = build_snapshot(&snapshot, &caps).expect("policy");
        assert_eq!(view.entries[0].presence, Presence::Current);
        assert_eq!(
            view.entries[0].account_id,
            format!("linux:uid:{}", own_uid())
        );
        eprintln!(
            "entries={} sessions={} current_session={} directory_complete={} sign_in={:?} same_user={:?} lock={:?} surface={:?}",
            view.entries.len(),
            snapshot.sessions.len(),
            snapshot.current_session.is_some(),
            snapshot.directory_complete,
            caps.system_sign_in,
            caps.same_user_session_switch,
            caps.lock_screen,
            caps.sign_in_surface,
        );
    }
}
