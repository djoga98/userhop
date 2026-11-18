use std::collections::BTreeMap;

use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::models::{
    AccountEntry, AccountSnapshot, Capability, Presence, SessionKind, SessionSummary,
    SwitchAvailability,
};

use super::ids::{ValidatedRequest, bounded_label};
use super::{
    AccountKey, AccountRecord, ProviderCapabilities, SessionKey, SessionRecord, SystemSnapshot,
};

pub const MAX_ENTRIES: usize = 256;
const MAX_LABEL_CHARS: usize = 96;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchPlan {
    NoOpCurrentSession,
    ActivateOwnSession(SessionKey),
    SystemSignIn,
}

fn summary(session: &SessionRecord) -> SessionSummary {
    SessionSummary {
        id: session.key.0.clone(),
        state: session.state,
        kind: session.kind,
        seat_id: session.seat.clone(),
    }
}

fn label_for(account: &AccountRecord) -> (String, String) {
    let user_name = bounded_label(&account.user_name, MAX_LABEL_CHARS);
    let display_name = account
        .display_name
        .as_deref()
        .map(|name| bounded_label(name, MAX_LABEL_CHARS))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| user_name.clone());
    (display_name, user_name)
}

fn availability_from(capability: Capability, available: SwitchAvailability) -> SwitchAvailability {
    match capability {
        Capability::Available => available,
        Capability::Unavailable { reason } => SwitchAvailability::Unavailable { reason },
    }
}

fn other_account_presence(sessions: &[&SessionRecord]) -> Presence {
    if sessions
        .iter()
        .any(|s| s.kind == SessionKind::LocalGraphical)
    {
        Presence::SignedIn
    } else if sessions.iter().any(|s| s.kind == SessionKind::Remote) {
        Presence::SignedInRemote
    } else {
        Presence::NotSignedIn
    }
}

pub fn build_snapshot(
    snapshot: &SystemSnapshot,
    capabilities: &ProviderCapabilities,
) -> SwitcherResult<AccountSnapshot> {
    let mut accounts: BTreeMap<&AccountKey, &AccountRecord> = BTreeMap::new();
    for account in &snapshot.accounts {
        accounts.entry(&account.key).or_insert(account);
    }
    let current = accounts
        .get(&snapshot.current_account)
        .copied()
        .ok_or_else(|| SwitcherError::new(ErrorCode::OsApiFailure))?;

    let mut sessions_by_account: BTreeMap<&AccountKey, Vec<&SessionRecord>> = BTreeMap::new();
    for session in &snapshot.sessions {
        if accounts.contains_key(&session.account) {
            sessions_by_account
                .entry(&session.account)
                .or_default()
                .push(session);
        }
    }

    let current_session = snapshot
        .current_session
        .as_ref()
        .and_then(|key| snapshot.sessions.iter().find(|s| &s.key == key));
    let current_seat = current_session.and_then(|s| s.seat.as_deref());
    let own_sessions = sessions_by_account
        .get(&current.key)
        .map(Vec::as_slice)
        .unwrap_or_default();

    let (current_display, current_user) = label_for(current);
    let mut entries = vec![AccountEntry {
        key: format!("current:{}", current.key.0),
        account_id: current.key.0.clone(),
        session_id: current_session.map(|s| s.key.0.clone()),
        display_name: current_display.clone(),
        user_name: current_user.clone(),
        is_current_user: true,
        presence: Presence::Current,
        sessions: own_sessions.iter().map(|s| summary(s)).collect(),
        availability: SwitchAvailability::Current,
        recent: false,
    }];

    for session in own_sessions.iter().filter(|s| {
        Some(&s.key) != current_session.map(|c| &c.key) && s.kind == SessionKind::LocalGraphical
    }) {
        let same_seat = current_seat.is_some() && session.seat.as_deref() == current_seat;
        let availability = if same_seat {
            availability_from(
                capabilities.same_user_session_switch,
                SwitchAvailability::Direct {
                    session_id: session.key.0.clone(),
                },
            )
        } else {
            SwitchAvailability::Unavailable {
                reason: ErrorCode::NotLocalGraphical,
            }
        };
        entries.push(AccountEntry {
            key: format!("session:{}", session.key.0),
            account_id: current.key.0.clone(),
            session_id: Some(session.key.0.clone()),
            display_name: current_display.clone(),
            user_name: current_user.clone(),
            is_current_user: true,
            presence: Presence::OwnSession,
            sessions: vec![summary(session)],
            availability,
            recent: false,
        });
    }

    let mut others: Vec<AccountEntry> = accounts
        .values()
        .filter(|account| account.key != current.key)
        .map(|account| {
            let sessions = sessions_by_account
                .get(&account.key)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let (display_name, user_name) = label_for(account);
            AccountEntry {
                key: format!("account:{}", account.key.0),
                account_id: account.key.0.clone(),
                session_id: None,
                display_name,
                user_name,
                is_current_user: false,
                presence: other_account_presence(sessions),
                sessions: sessions.iter().map(|s| summary(s)).collect(),
                availability: availability_from(
                    capabilities.system_sign_in,
                    SwitchAvailability::SystemSignIn,
                ),
                recent: false,
            }
        })
        .collect();
    others.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
            .then_with(|| a.user_name.to_lowercase().cmp(&b.user_name.to_lowercase()))
            .then_with(|| a.account_id.cmp(&b.account_id))
    });
    entries.extend(others);
    entries.truncate(MAX_ENTRIES);

    Ok(AccountSnapshot {
        entries,
        directory_complete: snapshot.directory_complete,
    })
}

/// Marks the account the user last switched to and lists it first after the user's own entries.
pub fn mark_recent(view: &mut AccountSnapshot, recent: &AccountKey) {
    let Some(index) = view
        .entries
        .iter()
        .position(|e| !e.is_current_user && e.account_id == recent.0)
    else {
        return;
    };
    let mut entry = view.entries.remove(index);
    entry.recent = true;
    let insert_at = view
        .entries
        .iter()
        .position(|e| !e.is_current_user)
        .unwrap_or(view.entries.len());
    view.entries.insert(insert_at, entry);
}

/// Resolves a UI request against a fresh OS snapshot; the UI's own view is never trusted.
pub fn plan_switch(
    snapshot: &SystemSnapshot,
    capabilities: &ProviderCapabilities,
    request: &ValidatedRequest,
) -> SwitcherResult<SwitchPlan> {
    let view = build_snapshot(snapshot, capabilities)?;
    let mut candidates = view
        .entries
        .iter()
        .filter(|entry| entry.account_id == request.account.0)
        .peekable();
    if candidates.peek().is_none() {
        return Err(SwitcherError::new(ErrorCode::SessionNotFound));
    }
    let entry = match &request.session {
        Some(session) => candidates
            .find(|entry| entry.session_id.as_deref() == Some(session.0.as_str()))
            .ok_or_else(|| SwitcherError::new(ErrorCode::SessionNoLongerActive))?,
        None => candidates
            .find(|entry| entry.presence != Presence::OwnSession)
            .ok_or_else(|| SwitcherError::new(ErrorCode::SessionNotFound))?,
    };
    match &entry.availability {
        SwitchAvailability::Current => Ok(SwitchPlan::NoOpCurrentSession),
        SwitchAvailability::Direct { session_id } => Ok(SwitchPlan::ActivateOwnSession(
            SessionKey(session_id.clone()),
        )),
        SwitchAvailability::SystemSignIn => Ok(SwitchPlan::SystemSignIn),
        SwitchAvailability::Unavailable { reason } => Err(SwitcherError::new(*reason)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SessionState, SignInSurface};
    use crate::session::fake::{account, caps_all, session};

    fn base() -> SystemSnapshot {
        SystemSnapshot {
            current_account: AccountKey("t:1000".into()),
            current_session: Some(SessionKey("t:s:2".into())),
            accounts: vec![
                account("t:1000", "ana", Some("Ana")),
                account("t:1001", "zoran", Some("Zoran")),
                account("t:1002", "bojan", None),
            ],
            sessions: vec![
                session(
                    "t:s:2",
                    "t:1000",
                    SessionKind::LocalGraphical,
                    Some("seat0"),
                ),
                session(
                    "t:s:5",
                    "t:1001",
                    SessionKind::LocalGraphical,
                    Some("seat0"),
                ),
            ],
            directory_complete: true,
        }
    }

    fn req(account: &str, session: Option<&str>) -> ValidatedRequest {
        ValidatedRequest {
            account: AccountKey(account.into()),
            session: session.map(|s| SessionKey(s.into())),
        }
    }

    #[test]
    fn current_user_first_then_others_sorted_by_display_name() {
        let view = build_snapshot(&base(), &caps_all()).expect("snapshot");
        let names: Vec<_> = view
            .entries
            .iter()
            .map(|e| e.display_name.as_str())
            .collect();
        assert_eq!(names, ["Ana", "bojan", "Zoran"]);
        assert!(view.entries[0].is_current_user);
        assert_eq!(view.entries[0].presence, Presence::Current);
        assert_eq!(view.entries[0].availability, SwitchAvailability::Current);
    }

    #[test]
    fn presence_comes_only_from_reported_sessions() {
        let view = build_snapshot(&base(), &caps_all()).expect("snapshot");
        let by_name = |n: &str| view.entries.iter().find(|e| e.display_name == n).expect(n);
        assert_eq!(by_name("Zoran").presence, Presence::SignedIn);
        assert_eq!(by_name("bojan").presence, Presence::NotSignedIn);
        assert!(by_name("bojan").sessions.is_empty());
    }

    #[test]
    fn remote_and_tty_sessions_never_count_as_local_sign_in() {
        let mut snap = base();
        snap.sessions
            .push(session("t:s:9", "t:1002", SessionKind::Remote, None));
        snap.sessions
            .push(session("t:s:10", "t:1002", SessionKind::Other, None));
        let view = build_snapshot(&snap, &caps_all()).expect("snapshot");
        let bojan = view
            .entries
            .iter()
            .find(|e| e.user_name == "bojan")
            .expect("bojan");
        assert_eq!(bojan.presence, Presence::SignedInRemote);
        assert_eq!(bojan.availability, SwitchAvailability::SystemSignIn);
    }

    #[test]
    fn other_users_never_get_direct_activation() {
        let view = build_snapshot(&base(), &caps_all()).expect("snapshot");
        for entry in view.entries.iter().filter(|e| !e.is_current_user) {
            assert!(!matches!(
                entry.availability,
                SwitchAvailability::Direct { .. }
            ));
        }
        let plan = plan_switch(&base(), &caps_all(), &req("t:1001", None)).expect("plan");
        assert_eq!(plan, SwitchPlan::SystemSignIn);
    }

    #[test]
    fn foreign_session_id_cannot_be_smuggled_into_direct_activation() {
        let err = plan_switch(&base(), &caps_all(), &req("t:1000", Some("t:s:5"))).unwrap_err();
        assert_eq!(err.code, ErrorCode::SessionNoLongerActive);
        let err = plan_switch(&base(), &caps_all(), &req("t:1001", Some("t:s:5"))).unwrap_err();
        assert_eq!(err.code, ErrorCode::SessionNoLongerActive);
    }

    #[test]
    fn own_second_session_on_same_seat_is_direct() {
        let mut snap = base();
        snap.sessions.push(session(
            "t:s:7",
            "t:1000",
            SessionKind::LocalGraphical,
            Some("seat0"),
        ));
        let plan = plan_switch(&snap, &caps_all(), &req("t:1000", Some("t:s:7"))).expect("plan");
        assert_eq!(
            plan,
            SwitchPlan::ActivateOwnSession(SessionKey("t:s:7".into()))
        );
        let view = build_snapshot(&snap, &caps_all()).expect("snapshot");
        assert_eq!(view.entries[1].presence, Presence::OwnSession);
    }

    #[test]
    fn own_session_on_other_seat_or_without_capability_is_unavailable() {
        let mut snap = base();
        snap.sessions.push(session(
            "t:s:7",
            "t:1000",
            SessionKind::LocalGraphical,
            Some("seat1"),
        ));
        let err = plan_switch(&snap, &caps_all(), &req("t:1000", Some("t:s:7"))).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotLocalGraphical);

        let mut snap = base();
        snap.sessions.push(session(
            "t:s:7",
            "t:1000",
            SessionKind::LocalGraphical,
            Some("seat0"),
        ));
        let mut caps = caps_all();
        caps.same_user_session_switch = Capability::unavailable(ErrorCode::UnsupportedPlatform);
        let err = plan_switch(&snap, &caps, &req("t:1000", Some("t:s:7"))).unwrap_err();
        assert_eq!(err.code, ErrorCode::UnsupportedPlatform);
    }

    #[test]
    fn own_remote_or_tty_sessions_are_not_offered() {
        let mut snap = base();
        snap.sessions
            .push(session("t:s:8", "t:1000", SessionKind::Other, None));
        let view = build_snapshot(&snap, &caps_all()).expect("snapshot");
        assert!(
            view.entries
                .iter()
                .all(|e| e.presence != Presence::OwnSession)
        );
        let err = plan_switch(&snap, &caps_all(), &req("t:1000", Some("t:s:8"))).unwrap_err();
        assert_eq!(err.code, ErrorCode::SessionNoLongerActive);
    }

    #[test]
    fn selecting_current_is_a_no_op() {
        let plan = plan_switch(&base(), &caps_all(), &req("t:1000", None)).expect("plan");
        assert_eq!(plan, SwitchPlan::NoOpCurrentSession);
        let plan = plan_switch(&base(), &caps_all(), &req("t:1000", Some("t:s:2"))).expect("plan");
        assert_eq!(plan, SwitchPlan::NoOpCurrentSession);
    }

    #[test]
    fn stale_account_is_rejected() {
        let err = plan_switch(&base(), &caps_all(), &req("t:4242", None)).unwrap_err();
        assert_eq!(err.code, ErrorCode::SessionNotFound);
    }

    #[test]
    fn session_that_ended_after_listing_is_rejected() {
        let mut snap = base();
        snap.sessions.push(session(
            "t:s:7",
            "t:1000",
            SessionKind::LocalGraphical,
            Some("seat0"),
        ));
        build_snapshot(&snap, &caps_all()).expect("listed while present");
        snap.sessions.retain(|s| s.key.0 != "t:s:7");
        let err = plan_switch(&snap, &caps_all(), &req("t:1000", Some("t:s:7"))).unwrap_err();
        assert_eq!(err.code, ErrorCode::SessionNoLongerActive);
    }

    #[test]
    fn unavailable_sign_in_carries_reason() {
        let mut caps = caps_all();
        caps.system_sign_in = Capability::unavailable(ErrorCode::RemoteSession);
        caps.sign_in_surface = SignInSurface::None;
        let view = build_snapshot(&base(), &caps).expect("snapshot");
        assert_eq!(
            view.entries[1].availability,
            SwitchAvailability::Unavailable {
                reason: ErrorCode::RemoteSession
            }
        );
        let err = plan_switch(&base(), &caps, &req("t:1001", None)).unwrap_err();
        assert_eq!(err.code, ErrorCode::RemoteSession);
    }

    #[test]
    fn missing_current_account_is_an_os_failure_not_a_guess() {
        let mut snap = base();
        snap.accounts.retain(|a| a.key.0 != "t:1000");
        assert_eq!(
            build_snapshot(&snap, &caps_all()).unwrap_err().code,
            ErrorCode::OsApiFailure
        );
    }

    #[test]
    fn sessions_of_unknown_accounts_are_ignored_and_duplicates_collapse() {
        let mut snap = base();
        snap.sessions.push(session(
            "t:s:11",
            "t:9999",
            SessionKind::LocalGraphical,
            Some("seat0"),
        ));
        snap.accounts.push(account("t:1001", "zoran-dup", None));
        let view = build_snapshot(&snap, &caps_all()).expect("snapshot");
        assert_eq!(view.entries.len(), 3);
        assert!(view.entries.iter().all(|e| e.account_id != "t:9999"));
    }

    #[test]
    fn labels_fall_back_to_user_name_and_are_sanitized() {
        let mut snap = base();
        snap.accounts
            .push(account("t:1003", "mila", Some(" \u{0007} ")));
        let view = build_snapshot(&snap, &caps_all()).expect("snapshot");
        assert!(view.entries.iter().any(|e| e.display_name == "mila"));
    }

    #[test]
    fn entries_are_bounded() {
        let mut snap = base();
        for i in 0..400 {
            snap.accounts
                .push(account(&format!("t:x{i}"), &format!("u{i}"), None));
        }
        assert_eq!(
            build_snapshot(&snap, &caps_all())
                .expect("snapshot")
                .entries
                .len(),
            MAX_ENTRIES
        );
    }

    #[test]
    fn recent_account_moves_up_and_is_marked_once() {
        let mut snap = base();
        snap.sessions.push(session(
            "t:s:7",
            "t:1000",
            SessionKind::LocalGraphical,
            Some("seat0"),
        ));
        let mut view = build_snapshot(&snap, &caps_all()).expect("view");
        mark_recent(&mut view, &AccountKey("t:1001".into()));
        let order: Vec<_> = view
            .entries
            .iter()
            .map(|e| e.display_name.as_str())
            .collect();
        assert_eq!(order, ["Ana", "Ana", "Zoran", "bojan"]);
        assert_eq!(view.entries.iter().filter(|e| e.recent).count(), 1);
        assert!(view.entries[2].recent);
    }

    #[test]
    fn recent_never_marks_the_current_user_or_unknown_accounts() {
        let mut view = build_snapshot(&base(), &caps_all()).expect("view");
        let before = view.clone();
        mark_recent(&mut view, &AccountKey("t:1000".into()));
        mark_recent(&mut view, &AccountKey("t:9999".into()));
        assert_eq!(view, before);
    }

    #[test]
    fn current_session_state_is_reported() {
        let view = build_snapshot(&base(), &caps_all()).expect("snapshot");
        assert_eq!(view.entries[0].session_id.as_deref(), Some("t:s:2"));
        assert_eq!(view.entries[0].sessions[0].state, SessionState::Active);
    }
}
