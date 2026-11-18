use crate::errors::ErrorCode;
use crate::models::{Capability, SessionKind, SessionState, SignInSurface};
use crate::session::{
    AccountKey, AccountRecord, ProviderCapabilities, SessionKey, SessionRecord, SystemSnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLogindSession {
    pub id: String,
    pub uid: u32,
    pub user_name: String,
    pub seat: Option<String>,
    pub class: String,
    pub session_type: String,
    pub remote: bool,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLinuxAccount {
    pub uid: u32,
    pub user_name: String,
    pub real_name: Option<String>,
    pub system_account: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLinuxState {
    pub own_uid: u32,
    pub own_user_name: Option<String>,
    pub own_session_id: Option<String>,
    pub sessions: Vec<RawLogindSession>,
    pub accounts: Option<Vec<RawLinuxAccount>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayManager {
    Gdm,
    Freedesktop,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLinuxEnvironment {
    pub logind_available: bool,
    pub own_session: Option<RawLogindSession>,
    pub seat_can_multi_session: bool,
    pub display_manager: DisplayManager,
    pub accounts_service: bool,
}

pub fn account_key(uid: u32) -> AccountKey {
    AccountKey(format!("linux:uid:{uid}"))
}

pub fn session_key(id: &str) -> SessionKey {
    SessionKey(format!("linux:session:{id}"))
}

pub fn session_id_from_key(key: &SessionKey) -> Option<&str> {
    key.0
        .strip_prefix("linux:session:")
        .filter(|id| is_valid_session_id(id))
}

pub fn is_valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn is_graphical_type(session_type: &str) -> bool {
    matches!(session_type, "x11" | "wayland" | "mir")
}

/// Only user-class sessions represent people; greeter, lock-screen and manager sessions do not.
pub fn is_user_session(session: &RawLogindSession) -> bool {
    session.class == "user" && session.state != "closing" && is_valid_session_id(&session.id)
}

pub fn is_local_graphical(session: &RawLogindSession) -> bool {
    is_user_session(session)
        && !session.remote
        && session.seat.is_some()
        && is_graphical_type(&session.session_type)
}

fn kind_of(session: &RawLogindSession) -> SessionKind {
    if session.remote {
        SessionKind::Remote
    } else if is_local_graphical(session) {
        SessionKind::LocalGraphical
    } else {
        SessionKind::Other
    }
}

fn state_of(state: &str) -> SessionState {
    match state {
        "active" => SessionState::Active,
        "online" => SessionState::Inactive,
        _ => SessionState::Unknown,
    }
}

pub fn assemble_snapshot(raw: &RawLinuxState) -> SystemSnapshot {
    let current_account = account_key(raw.own_uid);
    let mut accounts: Vec<AccountRecord> = Vec::new();
    let own_directory = raw.accounts.iter().flatten().find(|a| a.uid == raw.own_uid);
    let own_name = own_directory
        .map(|a| a.user_name.clone())
        .or_else(|| raw.own_user_name.clone())
        .or_else(|| {
            raw.sessions
                .iter()
                .find(|s| s.uid == raw.own_uid)
                .map(|s| s.user_name.clone())
        })
        .unwrap_or_else(|| raw.own_uid.to_string());
    accounts.push(AccountRecord {
        key: current_account.clone(),
        user_name: own_name,
        display_name: own_directory.and_then(|a| a.real_name.clone()),
    });

    for account in raw.accounts.iter().flatten() {
        if account.uid == raw.own_uid || account.system_account || account.locked {
            continue;
        }
        accounts.push(AccountRecord {
            key: account_key(account.uid),
            user_name: account.user_name.clone(),
            display_name: account.real_name.clone().filter(|n| !n.trim().is_empty()),
        });
    }

    let mut sessions = Vec::new();
    for session in raw.sessions.iter().filter(|s| is_user_session(s)) {
        let account = account_key(session.uid);
        if !accounts.iter().any(|a| a.key == account) {
            if raw.accounts.is_some() && kind_of(session) != SessionKind::LocalGraphical {
                continue;
            }
            accounts.push(AccountRecord {
                key: account.clone(),
                user_name: session.user_name.clone(),
                display_name: None,
            });
        }
        sessions.push(SessionRecord {
            key: session_key(&session.id),
            account,
            state: state_of(&session.state),
            kind: kind_of(session),
            seat: session.seat.clone(),
        });
    }

    let current_session = raw
        .own_session_id
        .as_deref()
        .map(session_key)
        .filter(|key| {
            sessions
                .iter()
                .any(|s| &s.key == key && s.account == current_account)
        });

    SystemSnapshot {
        current_account,
        current_session,
        accounts,
        sessions,
        directory_complete: raw.accounts.is_some(),
    }
}

pub fn capabilities(env: &RawLinuxEnvironment) -> ProviderCapabilities {
    let unavailable = Capability::unavailable;
    if !env.logind_available {
        let missing = unavailable(ErrorCode::UnsupportedDesktop);
        return ProviderCapabilities {
            session_discovery: missing,
            account_directory: missing,
            system_sign_in: missing,
            sign_in_surface: SignInSurface::None,
            same_user_session_switch: missing,
            lock_screen: missing,
        };
    }
    let own_local = env.own_session.as_ref().is_some_and(is_local_graphical);
    let system_sign_in = if env.own_session.as_ref().is_some_and(|s| s.remote) {
        unavailable(ErrorCode::RemoteSession)
    } else if !own_local {
        unavailable(ErrorCode::NotLocalGraphical)
    } else if !env.seat_can_multi_session {
        unavailable(ErrorCode::UnsupportedDesktop)
    } else if env.display_manager == DisplayManager::None {
        unavailable(ErrorCode::UnsupportedLoginManager)
    } else {
        Capability::Available
    };
    let sign_in_surface = match (system_sign_in, env.display_manager) {
        (Capability::Available, DisplayManager::Gdm) => SignInSurface::Gdm,
        (Capability::Available, DisplayManager::Freedesktop) => {
            SignInSurface::FreedesktopDisplayManager
        }
        _ => SignInSurface::None,
    };
    ProviderCapabilities {
        session_discovery: Capability::Available,
        account_directory: if env.accounts_service {
            Capability::Available
        } else {
            unavailable(ErrorCode::UnsupportedDesktop)
        },
        system_sign_in,
        sign_in_surface,
        same_user_session_switch: if own_local {
            Capability::Available
        } else {
            unavailable(ErrorCode::NotLocalGraphical)
        },
        lock_screen: match &env.own_session {
            None => unavailable(ErrorCode::SessionNotFound),
            Some(_) if !own_local => unavailable(ErrorCode::NotLocalGraphical),
            Some(_) => Capability::Available,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Presence, SwitchAvailability};
    use crate::session::policy::build_snapshot;

    fn logind(
        id: &str,
        uid: u32,
        user: &str,
        class: &str,
        kind: &str,
        seat: Option<&str>,
    ) -> RawLogindSession {
        RawLogindSession {
            id: id.into(),
            uid,
            user_name: user.into(),
            seat: seat.map(str::to_owned),
            class: class.into(),
            session_type: kind.into(),
            remote: false,
            state: "online".into(),
        }
    }

    fn directory(uid: u32, user: &str, real: Option<&str>) -> RawLinuxAccount {
        RawLinuxAccount {
            uid,
            user_name: user.into(),
            real_name: real.map(str::to_owned),
            system_account: false,
            locked: false,
        }
    }

    fn raw() -> RawLinuxState {
        RawLinuxState {
            own_uid: 1000,
            own_user_name: Some("ana".into()),
            own_session_id: Some("2".into()),
            sessions: vec![
                RawLogindSession {
                    state: "active".into(),
                    ..logind("2", 1000, "ana", "user", "wayland", Some("seat0"))
                },
                logind("c1", 120, "gdm", "greeter", "wayland", Some("seat0")),
                logind("5", 1001, "zoran", "user", "x11", Some("seat0")),
                logind("7", 1002, "mila", "user", "tty", None),
            ],
            accounts: Some(vec![
                directory(1000, "ana", Some("Ana")),
                directory(1001, "zoran", Some("Zoran")),
                directory(1002, "mila", None),
                RawLinuxAccount {
                    system_account: true,
                    ..directory(998, "builder", None)
                },
                RawLinuxAccount {
                    locked: true,
                    ..directory(1003, "old", None)
                },
            ]),
        }
    }

    fn env() -> RawLinuxEnvironment {
        RawLinuxEnvironment {
            logind_available: true,
            own_session: Some(logind("2", 1000, "ana", "user", "wayland", Some("seat0"))),
            seat_can_multi_session: true,
            display_manager: DisplayManager::Gdm,
            accounts_service: true,
        }
    }

    #[test]
    fn greeter_system_and_locked_accounts_are_excluded() {
        let snap = assemble_snapshot(&raw());
        let names: Vec<_> = snap.accounts.iter().map(|a| a.user_name.as_str()).collect();
        assert_eq!(names, ["ana", "zoran", "mila"]);
        assert!(snap.sessions.iter().all(|s| s.key.0 != "linux:session:c1"));
        assert_eq!(snap.current_session, Some(session_key("2")));
    }

    #[test]
    fn tty_session_does_not_mark_user_signed_in_graphically() {
        let view = build_snapshot(&assemble_snapshot(&raw()), &capabilities(&env())).expect("view");
        let mila = view
            .entries
            .iter()
            .find(|e| e.user_name == "mila")
            .expect("mila");
        assert_eq!(mila.presence, Presence::NotSignedIn);
        let zoran = view
            .entries
            .iter()
            .find(|e| e.user_name == "zoran")
            .expect("zoran");
        assert_eq!(zoran.presence, Presence::SignedIn);
        assert_eq!(zoran.availability, SwitchAvailability::SystemSignIn);
    }

    #[test]
    fn cross_user_session_is_never_direct_even_on_same_seat() {
        let view = build_snapshot(&assemble_snapshot(&raw()), &capabilities(&env())).expect("view");
        assert!(
            view.entries
                .iter()
                .filter(|e| !e.is_current_user)
                .all(|e| !matches!(e.availability, SwitchAvailability::Direct { .. }))
        );
    }

    #[test]
    fn own_second_graphical_session_is_direct() {
        let mut state = raw();
        state
            .sessions
            .push(logind("9", 1000, "ana", "user", "x11", Some("seat0")));
        let view = build_snapshot(&assemble_snapshot(&state), &capabilities(&env())).expect("view");
        let own = view
            .entries
            .iter()
            .find(|e| e.presence == Presence::OwnSession)
            .expect("own");
        assert_eq!(
            own.availability,
            SwitchAvailability::Direct {
                session_id: "linux:session:9".into()
            }
        );
    }

    #[test]
    fn without_accounts_service_lists_only_session_users() {
        let mut state = raw();
        state.accounts = None;
        let snap = assemble_snapshot(&state);
        assert!(!snap.directory_complete);
        let names: Vec<_> = snap.accounts.iter().map(|a| a.user_name.as_str()).collect();
        assert_eq!(names, ["ana", "zoran", "mila"]);
    }

    #[test]
    fn non_graphical_own_session_like_wsl_or_ssh_disables_handoff() {
        let mut tty = env();
        tty.own_session = Some(logind("1", 1000, "ana", "user", "tty", None));
        let caps = capabilities(&tty);
        assert_eq!(
            caps.system_sign_in,
            Capability::unavailable(ErrorCode::NotLocalGraphical)
        );
        assert_eq!(
            caps.same_user_session_switch,
            Capability::unavailable(ErrorCode::NotLocalGraphical)
        );
        assert_eq!(caps.sign_in_surface, SignInSurface::None);
        assert_eq!(
            caps.lock_screen,
            Capability::unavailable(ErrorCode::NotLocalGraphical)
        );
        assert!(capabilities(&env()).lock_screen.is_available());

        let mut ssh = env();
        ssh.own_session = Some(RawLogindSession {
            remote: true,
            ..logind("3", 1000, "ana", "user", "tty", None)
        });
        assert_eq!(
            capabilities(&ssh).system_sign_in,
            Capability::unavailable(ErrorCode::RemoteSession)
        );
    }

    #[test]
    fn missing_display_manager_or_multi_session_is_reported() {
        let mut no_dm = env();
        no_dm.display_manager = DisplayManager::None;
        assert_eq!(
            capabilities(&no_dm).system_sign_in,
            Capability::unavailable(ErrorCode::UnsupportedLoginManager)
        );
        let mut single = env();
        single.seat_can_multi_session = false;
        assert_eq!(
            capabilities(&single).system_sign_in,
            Capability::unavailable(ErrorCode::UnsupportedDesktop)
        );
        let mut freedesktop = env();
        freedesktop.display_manager = DisplayManager::Freedesktop;
        assert_eq!(
            capabilities(&freedesktop).sign_in_surface,
            SignInSurface::FreedesktopDisplayManager
        );
    }

    #[test]
    fn no_logind_means_nothing_is_offered() {
        let mut missing = env();
        missing.logind_available = false;
        let caps = capabilities(&missing);
        assert!(!caps.system_sign_in.is_available());
        assert!(!caps.lock_screen.is_available());
        assert!(!caps.session_discovery.is_available());
    }

    #[test]
    fn session_ids_are_validated_and_round_trip() {
        assert_eq!(session_id_from_key(&session_key("c12")), Some("c12"));
        assert_eq!(
            session_id_from_key(&SessionKey("linux:session:../x".into())),
            None
        );
        assert_eq!(
            session_id_from_key(&SessionKey("win:session:1".into())),
            None
        );
        let mut state = raw();
        state
            .sessions
            .push(logind("bad id", 1000, "ana", "user", "x11", Some("seat0")));
        assert!(
            assemble_snapshot(&state)
                .sessions
                .iter()
                .all(|s| !s.key.0.contains(' '))
        );
    }

    #[test]
    fn current_session_must_belong_to_current_uid() {
        let mut state = raw();
        state.own_session_id = Some("5".into());
        assert_eq!(assemble_snapshot(&state).current_session, None);
    }
}
