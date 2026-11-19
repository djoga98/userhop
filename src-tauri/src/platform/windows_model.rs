use crate::errors::ErrorCode;
use crate::models::{Capability, SessionKind, SessionState, SignInSurface};
use crate::session::{
    AccountKey, AccountRecord, ProviderCapabilities, SessionKey, SessionRecord, SystemSnapshot,
};

pub const PROTOCOL_CONSOLE: u16 = 0;
pub const PROTOCOL_RDP: u16 = 2;
pub const SERVICES_SESSION: u32 = 0;
pub const WINDOWS_11_FIRST_BUILD: u32 = 22000;

/// Windows 11 brings the DWM system backdrop and rounded corners for frameless windows.
pub fn is_windows_11(build: Option<u32>) -> bool {
    build.is_some_and(|build| build >= WINDOWS_11_FIRST_BUILD)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WtsState {
    Active,
    Connected,
    Disconnected,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSession {
    pub id: u32,
    pub state: WtsState,
    pub user_name: Option<String>,
    pub sid: Option<String>,
    pub protocol: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLocalAccount {
    pub name: String,
    pub full_name: Option<String>,
    pub sid: Option<String>,
    pub disabled: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawWindowsState {
    pub own_session_id: u32,
    pub current_user_sid: String,
    pub current_user_name: String,
    pub current_user_full_name: Option<String>,
    pub sessions: Vec<RawSession>,
    pub local_accounts: Option<Vec<RawLocalAccount>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawWindowsPolicy {
    pub own_session_id: u32,
    pub console_session_id: Option<u32>,
    pub own_protocol: Option<u16>,
    pub hide_fast_user_switching: bool,
    pub lock_disabled: bool,
    pub directory_available: bool,
}

pub fn account_key(sid: &str) -> AccountKey {
    AccountKey(format!("win:{sid}"))
}

pub fn session_key(id: u32) -> SessionKey {
    SessionKey(format!("win:session:{id}"))
}

fn is_valid_sid(sid: &str) -> bool {
    sid.len() <= 184
        && sid.starts_with("S-1-")
        && sid
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'-' || b == b'S')
}

fn session_account_key(session: &RawSession) -> AccountKey {
    match session.sid.as_deref().filter(|sid| is_valid_sid(sid)) {
        Some(sid) => account_key(sid),
        None => AccountKey(format!("win:session-user:{}", session.id)),
    }
}

fn state_of(state: WtsState) -> Option<SessionState> {
    match state {
        WtsState::Active => Some(SessionState::Active),
        WtsState::Disconnected => Some(SessionState::Inactive),
        WtsState::Connected => Some(SessionState::Unknown),
        WtsState::Other => None,
    }
}

fn kind_of(protocol: Option<u16>) -> SessionKind {
    match protocol {
        Some(PROTOCOL_CONSOLE) => SessionKind::LocalGraphical,
        Some(PROTOCOL_RDP) => SessionKind::Remote,
        _ => SessionKind::Other,
    }
}

pub fn assemble_snapshot(raw: &RawWindowsState) -> Option<SystemSnapshot> {
    if !is_valid_sid(&raw.current_user_sid) {
        return None;
    }
    let current_account = account_key(&raw.current_user_sid);
    let mut accounts = vec![AccountRecord {
        key: current_account.clone(),
        user_name: raw.current_user_name.clone(),
        display_name: raw.current_user_full_name.clone(),
    }];

    for local in raw.local_accounts.iter().flatten() {
        let Some(sid) = local.sid.as_deref().filter(|sid| is_valid_sid(sid)) else {
            continue;
        };
        if local.disabled || local.hidden || sid == raw.current_user_sid {
            continue;
        }
        accounts.push(AccountRecord {
            key: account_key(sid),
            user_name: local.name.clone(),
            display_name: local.full_name.clone().filter(|n| !n.trim().is_empty()),
        });
    }

    let mut sessions = Vec::new();
    for session in &raw.sessions {
        if session.id == SERVICES_SESSION {
            continue;
        }
        let Some(user_name) = session.user_name.as_deref().filter(|u| !u.is_empty()) else {
            continue;
        };
        let Some(state) = state_of(session.state) else {
            continue;
        };
        let account = if session.id == raw.own_session_id {
            current_account.clone()
        } else {
            session_account_key(session)
        };
        if !accounts.iter().any(|a| a.key == account) {
            accounts.push(AccountRecord {
                key: account.clone(),
                user_name: user_name.to_owned(),
                display_name: None,
            });
        }
        sessions.push(SessionRecord {
            key: session_key(session.id),
            account,
            state,
            kind: kind_of(session.protocol),
            seat: None,
        });
    }

    let current_session = sessions
        .iter()
        .find(|s| s.key == session_key(raw.own_session_id))
        .map(|s| s.key.clone());

    Some(SystemSnapshot {
        current_account,
        current_session,
        accounts,
        sessions,
        directory_complete: raw.local_accounts.is_some(),
    })
}

/// Account switching through the system sign-in screen is available on the local console.
pub fn capabilities(policy: &RawWindowsPolicy) -> ProviderCapabilities {
    let on_console = policy.console_session_id == Some(policy.own_session_id)
        && policy.own_protocol.is_none_or(|p| p == PROTOCOL_CONSOLE);
    let system_sign_in = if policy.hide_fast_user_switching || policy.lock_disabled {
        Capability::unavailable(ErrorCode::PolicyDisabled)
    } else if !on_console {
        Capability::unavailable(ErrorCode::RemoteSession)
    } else {
        Capability::Available
    };
    ProviderCapabilities {
        session_discovery: Capability::Available,
        account_directory: if policy.directory_available {
            Capability::Available
        } else {
            Capability::unavailable(ErrorCode::PermissionDenied)
        },
        system_sign_in,
        sign_in_surface: if system_sign_in.is_available() {
            SignInSurface::WindowsSignIn
        } else {
            SignInSurface::None
        },
        same_user_session_switch: Capability::unavailable(ErrorCode::UnsupportedPlatform),
        lock_screen: if policy.lock_disabled {
            Capability::unavailable(ErrorCode::PolicyDisabled)
        } else {
            Capability::Available
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Presence, SwitchAvailability};
    use crate::session::policy::build_snapshot;

    const ME: &str = "S-1-5-21-100-200-300-1001";
    const OTHER: &str = "S-1-5-21-100-200-300-1002";
    const IDLE: &str = "S-1-5-21-100-200-300-1003";

    fn local(name: &str, sid: &str) -> RawLocalAccount {
        RawLocalAccount {
            name: name.into(),
            full_name: None,
            sid: Some(sid.into()),
            disabled: false,
            hidden: false,
        }
    }

    fn raw() -> RawWindowsState {
        RawWindowsState {
            own_session_id: 1,
            current_user_sid: ME.into(),
            current_user_name: "ana".into(),
            current_user_full_name: Some("Ana M".into()),
            sessions: vec![
                RawSession {
                    id: 0,
                    state: WtsState::Disconnected,
                    user_name: None,
                    sid: None,
                    protocol: None,
                },
                RawSession {
                    id: 1,
                    state: WtsState::Active,
                    user_name: Some("ana".into()),
                    sid: Some(ME.into()),
                    protocol: Some(PROTOCOL_CONSOLE),
                },
                RawSession {
                    id: 2,
                    state: WtsState::Disconnected,
                    user_name: Some("zoran".into()),
                    sid: Some(OTHER.into()),
                    protocol: Some(PROTOCOL_CONSOLE),
                },
                RawSession {
                    id: 65536,
                    state: WtsState::Other,
                    user_name: None,
                    sid: None,
                    protocol: None,
                },
            ],
            local_accounts: Some(vec![
                local("ana", ME),
                local("zoran", OTHER),
                local("mila", IDLE),
                RawLocalAccount {
                    disabled: true,
                    ..local("Guest", "S-1-5-21-100-200-300-501")
                },
                RawLocalAccount {
                    hidden: true,
                    ..local("svc", "S-1-5-21-100-200-300-1100")
                },
            ]),
        }
    }

    fn policy() -> RawWindowsPolicy {
        RawWindowsPolicy {
            own_session_id: 1,
            console_session_id: Some(1),
            own_protocol: Some(PROTOCOL_CONSOLE),
            hide_fast_user_switching: false,
            lock_disabled: false,
            directory_available: true,
        }
    }

    #[test]
    fn identity_comes_from_token_sid_not_names() {
        let mut state = raw();
        state.local_accounts.as_mut().expect("accounts")[1].name = "ana".into();
        let snap = assemble_snapshot(&state).expect("snapshot");
        assert_eq!(snap.current_account, account_key(ME));
        assert_eq!(snap.current_session, Some(session_key(1)));
        let anas = snap
            .accounts
            .iter()
            .filter(|a| a.user_name == "ana")
            .count();
        assert_eq!(anas, 2, "a same-named account stays a separate identity");
    }

    #[test]
    fn excludes_services_listener_disabled_and_hidden_accounts() {
        let snap = assemble_snapshot(&raw()).expect("snapshot");
        let names: Vec<_> = snap.accounts.iter().map(|a| a.user_name.as_str()).collect();
        assert_eq!(names, ["ana", "zoran", "mila"]);
        assert_eq!(snap.sessions.len(), 2);
        assert!(snap.directory_complete);
    }

    #[test]
    fn fast_user_switching_rows() {
        let snap = assemble_snapshot(&raw()).expect("snapshot");
        let view = build_snapshot(&snap, &capabilities(&policy())).expect("view");
        let zoran = view
            .entries
            .iter()
            .find(|e| e.user_name == "zoran")
            .expect("zoran");
        assert_eq!(zoran.presence, Presence::SignedIn);
        assert_eq!(zoran.availability, SwitchAvailability::SystemSignIn);
        let mila = view
            .entries
            .iter()
            .find(|e| e.user_name == "mila")
            .expect("mila");
        assert_eq!(mila.presence, Presence::NotSignedIn);
        assert_eq!(view.entries[0].display_name, "Ana M");
    }

    #[test]
    fn rdp_sessions_are_remote_and_unresolved_users_get_opaque_ids() {
        let mut state = raw();
        state.sessions.push(RawSession {
            id: 3,
            state: WtsState::Active,
            user_name: Some("CORP\\remote".into()),
            sid: None,
            protocol: Some(PROTOCOL_RDP),
        });
        let snap = assemble_snapshot(&state).expect("snapshot");
        let session = snap
            .sessions
            .iter()
            .find(|s| s.key == session_key(3))
            .expect("rdp");
        assert_eq!(session.kind, SessionKind::Remote);
        assert_eq!(session.account.0, "win:session-user:3");
    }

    #[test]
    fn directory_failure_still_lists_signed_in_sessions() {
        let mut state = raw();
        state.local_accounts = None;
        let snap = assemble_snapshot(&state).expect("snapshot");
        assert!(!snap.directory_complete);
        assert!(snap.accounts.iter().any(|a| a.user_name == "zoran"));
        assert!(!snap.accounts.iter().any(|a| a.user_name == "mila"));
    }

    #[test]
    fn connecting_and_listener_states_are_unknown_or_skipped() {
        let mut state = raw();
        state.sessions[2].state = WtsState::Connected;
        let snap = assemble_snapshot(&state).expect("snapshot");
        let session = snap
            .sessions
            .iter()
            .find(|s| s.key == session_key(2))
            .expect("session");
        assert_eq!(session.state, SessionState::Unknown);
        assert!(snap.sessions.iter().all(|s| s.key != session_key(65536)));
    }

    #[test]
    fn invalid_current_sid_fails_closed() {
        let mut state = raw();
        state.current_user_sid = "ana".into();
        assert!(assemble_snapshot(&state).is_none());
    }

    #[test]
    fn remote_session_or_policy_disables_sign_in_handoff() {
        let mut remote = policy();
        remote.console_session_id = Some(5);
        assert_eq!(
            capabilities(&remote).system_sign_in,
            Capability::unavailable(ErrorCode::RemoteSession)
        );
        let mut rdp = policy();
        rdp.own_protocol = Some(PROTOCOL_RDP);
        assert_eq!(
            capabilities(&rdp).system_sign_in,
            Capability::unavailable(ErrorCode::RemoteSession)
        );
        let mut hidden = policy();
        hidden.hide_fast_user_switching = true;
        let caps = capabilities(&hidden);
        assert_eq!(
            caps.system_sign_in,
            Capability::unavailable(ErrorCode::PolicyDisabled)
        );
        assert_eq!(caps.sign_in_surface, SignInSurface::None);
        let mut no_lock = policy();
        no_lock.lock_disabled = true;
        let caps = capabilities(&no_lock);
        assert_eq!(
            caps.lock_screen,
            Capability::unavailable(ErrorCode::PolicyDisabled)
        );
        assert_eq!(
            caps.system_sign_in,
            Capability::unavailable(ErrorCode::PolicyDisabled)
        );
    }

    #[test]
    fn windows_10_builds_use_the_solid_fallback() {
        assert!(!is_windows_11(Some(19045)));
        assert!(!is_windows_11(Some(21996)));
        assert!(is_windows_11(Some(22000)));
        assert!(is_windows_11(Some(26300)));
        assert!(!is_windows_11(None));
    }

    #[test]
    fn windows_never_offers_direct_activation() {
        let caps = capabilities(&policy());
        assert!(!caps.same_user_session_switch.is_available());
        assert_eq!(caps.sign_in_surface, SignInSurface::WindowsSignIn);
    }
}
