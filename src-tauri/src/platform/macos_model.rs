use crate::errors::ErrorCode;
use crate::models::{Capability, SessionKind, SessionState, SignInSurface};
use crate::session::{
    AccountKey, AccountRecord, ProviderCapabilities, SessionKey, SessionRecord, SystemSnapshot,
};

/// First UID that macOS assigns to people; lower IDs are system or hidden accounts.
pub const FIRST_USER_UID: u32 = 501;
const CONSOLE_LINE: &str = "console";
const NO_LOGIN_SHELLS: [&str; 3] = ["/usr/bin/false", "/sbin/nologin", "/usr/sbin/nologin"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMacUser {
    pub uid: u32,
    pub name: String,
    pub full_name: Option<String>,
    pub shell: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLogin {
    pub uid: u32,
    pub name: String,
    pub line: String,
    pub host: String,
    pub pid: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMacState {
    pub own_uid: u32,
    pub own_name: String,
    pub own_full_name: Option<String>,
    pub console_uid: Option<u32>,
    pub users: Option<Vec<RawMacUser>>,
    pub logins: Vec<RawLogin>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawMacEnvironment {
    pub own_uid: u32,
    pub console_uid: Option<u32>,
    pub can_switch_to_login_window: bool,
    pub can_lock: bool,
    pub fast_user_switching: Option<bool>,
}

pub fn account_key(uid: u32) -> AccountKey {
    AccountKey(format!("mac:uid:{uid}"))
}

pub fn session_key(pid: i32) -> SessionKey {
    SessionKey(format!("mac:console:{pid}"))
}

pub fn is_person(user: &RawMacUser) -> bool {
    user.uid >= FIRST_USER_UID
        && !user.name.starts_with('_')
        && !NO_LOGIN_SHELLS.contains(&user.shell.as_str())
}

fn kind_of(login: &RawLogin) -> SessionKind {
    if login.line == CONSOLE_LINE {
        SessionKind::LocalGraphical
    } else if !login.host.is_empty() {
        SessionKind::Remote
    } else {
        SessionKind::Other
    }
}

pub fn assemble_snapshot(raw: &RawMacState) -> SystemSnapshot {
    let current_account = account_key(raw.own_uid);
    let mut accounts = vec![AccountRecord {
        key: current_account.clone(),
        user_name: raw.own_name.clone(),
        display_name: raw.own_full_name.clone(),
    }];
    for user in raw.users.iter().flatten() {
        if user.uid == raw.own_uid || !is_person(user) {
            continue;
        }
        accounts.push(AccountRecord {
            key: account_key(user.uid),
            user_name: user.name.clone(),
            display_name: user.full_name.clone().filter(|n| !n.trim().is_empty()),
        });
    }

    let mut sessions: Vec<SessionRecord> = Vec::new();
    for login in &raw.logins {
        let kind = kind_of(login);
        if kind == SessionKind::Other && login.uid != raw.own_uid {
            continue;
        }
        let account = account_key(login.uid);
        if !accounts.iter().any(|a| a.key == account) {
            if kind != SessionKind::LocalGraphical || login.uid < FIRST_USER_UID {
                continue;
            }
            accounts.push(AccountRecord {
                key: account.clone(),
                user_name: login.name.clone(),
                display_name: None,
            });
        }
        let key = if kind == SessionKind::LocalGraphical {
            session_key(login.pid)
        } else {
            SessionKey(format!("mac:tty:{}", login.pid))
        };
        if sessions.iter().any(|s| s.key == key) {
            continue;
        }
        let state = match (kind, raw.console_uid) {
            (SessionKind::LocalGraphical, Some(console)) if console == login.uid => {
                SessionState::Active
            }
            (SessionKind::LocalGraphical, Some(_)) => SessionState::Inactive,
            _ => SessionState::Unknown,
        };
        sessions.push(SessionRecord {
            key,
            account,
            state,
            kind,
            seat: None,
        });
    }

    let current_session = sessions
        .iter()
        .find(|s| s.account == current_account && s.kind == SessionKind::LocalGraphical)
        .map(|s| s.key.clone());

    SystemSnapshot {
        current_account,
        current_session,
        accounts,
        sessions,
        directory_complete: raw.users.is_some(),
    }
}

/// The login window is OS-owned; returning to any session always requires that user's password.
pub fn capabilities(env: &RawMacEnvironment) -> ProviderCapabilities {
    let on_console = env.console_uid == Some(env.own_uid);
    let system_sign_in = if !env.can_switch_to_login_window {
        Capability::unavailable(ErrorCode::UnsupportedDesktop)
    } else if env.fast_user_switching == Some(false) {
        Capability::unavailable(ErrorCode::PolicyDisabled)
    } else if !on_console {
        Capability::unavailable(ErrorCode::NotLocalGraphical)
    } else {
        Capability::Available
    };
    ProviderCapabilities {
        session_discovery: Capability::Available,
        account_directory: Capability::Available,
        system_sign_in,
        sign_in_surface: if system_sign_in.is_available() {
            SignInSurface::MacosLoginWindow
        } else {
            SignInSurface::None
        },
        same_user_session_switch: Capability::unavailable(ErrorCode::UnsupportedPlatform),
        lock_screen: if env.can_lock && on_console {
            Capability::Available
        } else if env.can_lock {
            Capability::unavailable(ErrorCode::NotLocalGraphical)
        } else {
            Capability::unavailable(ErrorCode::UnsupportedDesktop)
        },
    }
}

/// Reads `MultipleSessionEnabled` from the system-wide global preferences plist.
pub fn fast_user_switching_enabled(preferences: &plist::Value) -> Option<bool> {
    preferences
        .as_dictionary()?
        .get("MultipleSessionEnabled")?
        .as_boolean()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Presence, SwitchAvailability};
    use crate::session::policy::build_snapshot;

    fn user(uid: u32, name: &str, full: Option<&str>) -> RawMacUser {
        RawMacUser {
            uid,
            name: name.into(),
            full_name: full.map(str::to_owned),
            shell: "/bin/zsh".into(),
        }
    }

    fn login(uid: u32, name: &str, line: &str, host: &str, pid: i32) -> RawLogin {
        RawLogin {
            uid,
            name: name.into(),
            line: line.into(),
            host: host.into(),
            pid,
        }
    }

    fn raw() -> RawMacState {
        RawMacState {
            own_uid: 501,
            own_name: "ana".into(),
            own_full_name: Some("Ana".into()),
            console_uid: Some(501),
            users: Some(vec![
                user(501, "ana", Some("Ana")),
                user(502, "zoran", Some("Zoran")),
                user(503, "mila", None),
                user(248, "_securityagent", None),
                user(0, "root", Some("System Administrator")),
                RawMacUser {
                    shell: "/usr/bin/false".into(),
                    ..user(504, "kiosk", None)
                },
            ]),
            logins: vec![
                login(501, "ana", "console", "", 140),
                login(501, "ana", "ttys000", "", 900),
                login(502, "zoran", "console", "", 220),
                login(503, "mila", "ttys001", "10.0.0.5", 777),
            ],
        }
    }

    fn env() -> RawMacEnvironment {
        RawMacEnvironment {
            own_uid: 501,
            console_uid: Some(501),
            can_switch_to_login_window: true,
            can_lock: true,
            fast_user_switching: None,
        }
    }

    #[test]
    fn system_hidden_and_no_login_accounts_are_excluded() {
        let snap = assemble_snapshot(&raw());
        let names: Vec<_> = snap.accounts.iter().map(|a| a.user_name.as_str()).collect();
        assert_eq!(names, ["ana", "zoran", "mila"]);
        assert!(snap.directory_complete);
    }

    #[test]
    fn console_logins_are_graphical_and_ssh_is_remote() {
        let snap = assemble_snapshot(&raw());
        assert_eq!(snap.current_session, Some(session_key(140)));
        let view = build_snapshot(&snap, &capabilities(&env())).expect("view");
        let zoran = view
            .entries
            .iter()
            .find(|e| e.user_name == "zoran")
            .expect("zoran");
        assert_eq!(zoran.presence, Presence::SignedIn);
        assert_eq!(zoran.availability, SwitchAvailability::SystemSignIn);
        assert_eq!(zoran.sessions[0].state, SessionState::Inactive);
        let mila = view
            .entries
            .iter()
            .find(|e| e.user_name == "mila")
            .expect("mila");
        assert_eq!(mila.presence, Presence::SignedInRemote);
        assert_eq!(view.entries[0].sessions.len(), 2);
    }

    #[test]
    fn foreign_terminal_sessions_are_not_listed() {
        let mut state = raw();
        state.logins.push(login(503, "mila", "ttys004", "", 991));
        let snap = assemble_snapshot(&state);
        assert!(!snap.sessions.iter().any(|s| s.key.0 == "mac:tty:991"));
    }

    #[test]
    fn network_user_on_console_is_listed_but_system_logins_are_not() {
        let mut state = raw();
        state
            .logins
            .push(login(1_000_123, "corp.user", "console", "", 300));
        state.logins.push(login(0, "root", "console", "", 301));
        let snap = assemble_snapshot(&state);
        assert!(snap.accounts.iter().any(|a| a.user_name == "corp.user"));
        assert!(!snap.accounts.iter().any(|a| a.user_name == "root"));
    }

    #[test]
    fn directory_failure_still_lists_console_users() {
        let mut state = raw();
        state.users = None;
        let snap = assemble_snapshot(&state);
        assert!(!snap.directory_complete);
        assert!(snap.accounts.iter().any(|a| a.user_name == "zoran"));
        assert!(!snap.accounts.iter().any(|a| a.user_name == "mila"));
    }

    #[test]
    fn sign_in_requires_login_framework_console_and_policy() {
        assert!(capabilities(&env()).system_sign_in.is_available());
        assert_eq!(
            capabilities(&env()).sign_in_surface,
            SignInSurface::MacosLoginWindow
        );
        let missing = RawMacEnvironment {
            can_switch_to_login_window: false,
            ..env()
        };
        assert_eq!(
            capabilities(&missing).system_sign_in,
            Capability::unavailable(ErrorCode::UnsupportedDesktop)
        );
        let disabled = RawMacEnvironment {
            fast_user_switching: Some(false),
            ..env()
        };
        assert_eq!(
            capabilities(&disabled).system_sign_in,
            Capability::unavailable(ErrorCode::PolicyDisabled)
        );
        let background = RawMacEnvironment {
            console_uid: Some(502),
            ..env()
        };
        let caps = capabilities(&background);
        assert_eq!(
            caps.system_sign_in,
            Capability::unavailable(ErrorCode::NotLocalGraphical)
        );
        assert_eq!(
            caps.lock_screen,
            Capability::unavailable(ErrorCode::NotLocalGraphical)
        );
    }

    #[test]
    fn macos_never_offers_direct_activation() {
        let caps = capabilities(&env());
        assert!(!caps.same_user_session_switch.is_available());
        let view = build_snapshot(&assemble_snapshot(&raw()), &caps).expect("view");
        assert!(
            view.entries
                .iter()
                .all(|e| !matches!(e.availability, SwitchAvailability::Direct { .. }))
        );
    }

    #[test]
    fn reads_fast_user_switching_preference() {
        let mut dict = plist::Dictionary::new();
        assert_eq!(
            fast_user_switching_enabled(&plist::Value::Dictionary(dict.clone())),
            None
        );
        dict.insert(
            "MultipleSessionEnabled".into(),
            plist::Value::Boolean(false),
        );
        assert_eq!(
            fast_user_switching_enabled(&plist::Value::Dictionary(dict)),
            Some(false)
        );
        assert_eq!(
            fast_user_switching_enabled(&plist::Value::Boolean(true)),
            None
        );
    }
}
