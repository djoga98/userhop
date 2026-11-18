use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::models::{AccountSnapshot, HandoffMode, SwitchRequest, SwitchResult};

use super::ids::validate_request;
use super::policy::{SwitchPlan, build_snapshot, mark_recent, plan_switch};
use super::{AccountKey, ProviderCapabilities, SessionProvider};

/// Kept in memory only: the app keeps running in the original session while another account is
/// in use, so it is still known when the person returns, and nothing about other users is saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentTarget {
    pub account: AccountKey,
    pub label: String,
}

pub struct SessionService {
    provider: Arc<dyn SessionProvider>,
    in_flight: AtomicBool,
    recent: Mutex<Option<RecentTarget>>,
}

struct InFlightGuard<'a>(&'a AtomicBool);

impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl SessionService {
    pub fn new(provider: Arc<dyn SessionProvider>) -> Self {
        Self {
            provider,
            in_flight: AtomicBool::new(false),
            recent: Mutex::new(None),
        }
    }

    pub fn recent(&self) -> Option<RecentTarget> {
        self.recent.lock().ok().and_then(|r| r.clone())
    }

    fn remember(&self, target: Option<RecentTarget>) {
        if let (Some(target), Ok(mut slot)) = (target, self.recent.lock()) {
            *slot = Some(target);
        }
    }

    pub fn capabilities(&self) -> ProviderCapabilities {
        self.provider.capabilities()
    }

    pub fn accounts(&self) -> SwitcherResult<AccountSnapshot> {
        let snapshot = self.provider.snapshot()?;
        let mut view = build_snapshot(&snapshot, &self.provider.capabilities())?;
        if let Some(recent) = self.recent() {
            mark_recent(&mut view, &recent.account);
        }
        Ok(view)
    }

    fn begin(&self) -> SwitcherResult<InFlightGuard<'_>> {
        self.in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| InFlightGuard(&self.in_flight))
            .map_err(|_| SwitcherError::new(ErrorCode::Busy))
    }

    /// Plans against a fresh snapshot and runs at most one OS handoff at a time; never retries.
    pub fn request_switch(
        &self,
        request: &SwitchRequest,
        before_handoff: &dyn Fn(),
    ) -> SwitcherResult<SwitchResult> {
        let validated = validate_request(request)?;
        let _guard = self.begin()?;
        let snapshot = self.provider.snapshot()?;
        let plan = plan_switch(&snapshot, &self.provider.capabilities(), &validated)?;
        match plan {
            SwitchPlan::NoOpCurrentSession => Ok(SwitchResult::NoOpCurrentSession),
            SwitchPlan::ActivateOwnSession(session) => {
                before_handoff();
                self.provider.activate_own_session(&session)?;
                Ok(SwitchResult::HandoffRequested {
                    mode: HandoffMode::Direct,
                })
            }
            SwitchPlan::SystemSignIn => {
                let label = build_snapshot(&snapshot, &self.provider.capabilities())?
                    .entries
                    .into_iter()
                    .find(|e| e.account_id == validated.account.0)
                    .map(|e| e.display_name);
                before_handoff();
                self.provider.open_system_sign_in()?;
                self.remember(label.map(|label| RecentTarget {
                    account: validated.account.clone(),
                    label,
                }));
                Ok(SwitchResult::HandoffRequested {
                    mode: HandoffMode::SystemSignIn,
                })
            }
        }
    }

    /// Re-runs the full request path for the remembered account, including fresh validation.
    pub fn switch_to_recent(&self, before_handoff: &dyn Fn()) -> SwitcherResult<SwitchResult> {
        let recent = self
            .recent()
            .ok_or_else(|| SwitcherError::new(ErrorCode::SessionNotFound))?;
        self.request_switch(
            &SwitchRequest {
                account_id: recent.account.0,
                session_id: None,
            },
            before_handoff,
        )
    }

    pub fn open_system_sign_in(&self, before_handoff: &dyn Fn()) -> SwitcherResult<SwitchResult> {
        let _guard = self.begin()?;
        if let crate::models::Capability::Unavailable { reason } =
            self.provider.capabilities().system_sign_in
        {
            return Err(SwitcherError::new(reason));
        }
        before_handoff();
        self.provider.open_system_sign_in()?;
        Ok(SwitchResult::HandoffRequested {
            mode: HandoffMode::SystemSignIn,
        })
    }

    pub fn lock(&self, before_handoff: &dyn Fn()) -> SwitcherResult<()> {
        let _guard = self.begin()?;
        if let crate::models::Capability::Unavailable { reason } =
            self.provider.capabilities().lock_screen
        {
            return Err(SwitcherError::new(reason));
        }
        before_handoff();
        self.provider.lock_current_session()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;
    use std::sync::{Barrier, mpsc};
    use std::thread;

    use super::*;
    use crate::models::{Capability, SessionKind};
    use crate::session::fake::{Call, FakeProvider, account, session};
    use crate::session::{AccountKey, SessionKey, SystemSnapshot};

    fn snapshot() -> SystemSnapshot {
        SystemSnapshot {
            current_account: AccountKey("t:1".into()),
            current_session: Some(SessionKey("t:s:1".into())),
            accounts: vec![account("t:1", "ana", None), account("t:2", "zoran", None)],
            sessions: vec![session(
                "t:s:1",
                "t:1",
                SessionKind::LocalGraphical,
                Some("seat0"),
            )],
            directory_complete: true,
        }
    }

    fn request(account: &str) -> SwitchRequest {
        SwitchRequest {
            account_id: account.into(),
            session_id: None,
        }
    }

    #[test]
    fn other_account_hides_then_requests_sign_in_once() {
        let provider = Arc::new(FakeProvider::new(snapshot()));
        let service = SessionService::new(provider.clone());
        let hidden = AtomicUsize::new(0);
        let result = service
            .request_switch(&request("t:2"), &|| {
                hidden.fetch_add(1, Ordering::SeqCst);
            })
            .expect("switch");
        assert_eq!(
            result,
            SwitchResult::HandoffRequested {
                mode: HandoffMode::SystemSignIn
            }
        );
        assert_eq!(hidden.load(Ordering::SeqCst), 1);
        assert_eq!(provider.calls(), vec![Call::SignIn]);
    }

    #[test]
    fn current_account_does_not_touch_the_os() {
        let provider = Arc::new(FakeProvider::new(snapshot()));
        let service = SessionService::new(provider.clone());
        let result = service
            .request_switch(&request("t:1"), &|| {})
            .expect("noop");
        assert_eq!(result, SwitchResult::NoOpCurrentSession);
        assert!(provider.calls().is_empty());
    }

    #[test]
    fn concurrent_requests_execute_exactly_one_handoff() {
        let provider = Arc::new(FakeProvider::new(snapshot()));
        let service = Arc::new(SessionService::new(provider.clone()));
        let barrier = Arc::new(Barrier::new(2));
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let release_rx = std::sync::Mutex::new(release_rx);

        let first = {
            let service = service.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                service.request_switch(&request("t:2"), &|| {
                    barrier.wait();
                    release_rx.lock().expect("rx").recv().ok();
                })
            })
        };
        barrier.wait();
        let second = service.request_switch(&request("t:2"), &|| {});
        assert_eq!(second.unwrap_err().code, ErrorCode::Busy);
        release_tx.send(()).expect("release");
        assert!(first.join().expect("join").is_ok());
        assert_eq!(provider.calls(), vec![Call::SignIn]);
        assert!(service.request_switch(&request("t:1"), &|| {}).is_ok());
    }

    #[test]
    fn os_rejection_is_reported_and_not_retried() {
        let mut fake = FakeProvider::new(snapshot());
        fake.fail_actions = Some(SwitcherError::new(ErrorCode::PermissionDenied));
        let provider = Arc::new(fake);
        let service = SessionService::new(provider.clone());
        let err = service.request_switch(&request("t:2"), &|| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::PermissionDenied);
        assert_eq!(provider.calls(), vec![Call::SignIn]);
    }

    #[test]
    fn enumeration_failure_blocks_switching() {
        let provider = Arc::new(FakeProvider::new(snapshot()));
        *provider.snapshot.lock().expect("lock") =
            Err(SwitcherError::new(ErrorCode::PermissionDenied));
        let service = SessionService::new(provider.clone());
        let err = service.request_switch(&request("t:2"), &|| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::PermissionDenied);
        assert!(provider.calls().is_empty());
    }

    #[test]
    fn invalid_ids_are_rejected_before_os_access() {
        let provider = Arc::new(FakeProvider::new(snapshot()));
        let service = SessionService::new(provider.clone());
        let err = service
            .request_switch(&request("t:2 && reboot"), &|| {})
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidRequest);
        assert!(provider.calls().is_empty());
    }

    #[test]
    fn sign_in_and_lock_respect_capabilities() {
        let mut fake = FakeProvider::new(snapshot());
        fake.capabilities.system_sign_in = Capability::unavailable(ErrorCode::RemoteSession);
        fake.capabilities.lock_screen = Capability::unavailable(ErrorCode::PolicyDisabled);
        let provider = Arc::new(fake);
        let service = SessionService::new(provider.clone());
        assert_eq!(
            service.open_system_sign_in(&|| {}).unwrap_err().code,
            ErrorCode::RemoteSession
        );
        assert_eq!(
            service.lock(&|| {}).unwrap_err().code,
            ErrorCode::PolicyDisabled
        );
        assert!(provider.calls().is_empty());
    }

    #[test]
    fn successful_handoff_is_remembered_and_listed_first() {
        let provider = Arc::new(FakeProvider::new(snapshot()));
        let service = SessionService::new(provider.clone());
        assert!(service.recent().is_none());
        service
            .request_switch(&request("t:2"), &|| {})
            .expect("switch");
        let recent = service.recent().expect("recent");
        assert_eq!(recent.account.0, "t:2");
        assert_eq!(recent.label, "zoran");
        let view = service.accounts().expect("accounts");
        assert!(view.entries[1].recent);
        service.switch_to_recent(&|| {}).expect("switch back");
        assert_eq!(provider.calls(), vec![Call::SignIn, Call::SignIn]);
    }

    #[test]
    fn failed_or_no_op_switches_are_not_remembered() {
        let mut fake = FakeProvider::new(snapshot());
        fake.fail_actions = Some(SwitcherError::new(ErrorCode::PermissionDenied));
        let service = SessionService::new(Arc::new(fake));
        assert!(service.request_switch(&request("t:2"), &|| {}).is_err());
        assert!(service.request_switch(&request("t:1"), &|| {}).is_ok());
        assert!(service.recent().is_none());
        assert_eq!(
            service.switch_to_recent(&|| {}).unwrap_err().code,
            ErrorCode::SessionNotFound
        );
    }

    #[test]
    fn recent_account_that_disappeared_is_rejected_fresh() {
        let provider = Arc::new(FakeProvider::new(snapshot()));
        let service = SessionService::new(provider.clone());
        service
            .request_switch(&request("t:2"), &|| {})
            .expect("switch");
        provider
            .snapshot
            .lock()
            .expect("lock")
            .as_mut()
            .expect("snapshot")
            .accounts
            .retain(|a| a.key.0 != "t:2");
        assert_eq!(
            service.switch_to_recent(&|| {}).unwrap_err().code,
            ErrorCode::SessionNotFound
        );
        assert_eq!(provider.calls(), vec![Call::SignIn]);
    }

    #[test]
    fn lock_calls_provider_when_available() {
        let provider = Arc::new(FakeProvider::new(snapshot()));
        let service = SessionService::new(provider.clone());
        service.lock(&|| {}).expect("lock");
        assert_eq!(provider.calls(), vec![Call::Lock]);
    }
}
