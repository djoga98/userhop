#[cfg(any(target_os = "linux", test))]
pub mod linux_model;
#[cfg(any(windows, test))]
pub mod windows_model;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

use std::sync::Arc;

use crate::models::{Backdrop, Platform};
use crate::session::SessionProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    Changed,
    CurrentLocked,
    Resumed,
}

pub type EventSink = Arc<dyn Fn(SessionEvent) + Send + Sync>;

#[cfg(windows)]
pub fn create_provider() -> Arc<dyn SessionProvider> {
    Arc::new(windows::WindowsProvider)
}

#[cfg(target_os = "linux")]
pub fn create_provider() -> Arc<dyn SessionProvider> {
    Arc::new(linux::LinuxProvider::new())
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn create_provider() -> Arc<dyn SessionProvider> {
    Arc::new(unsupported::UnsupportedProvider)
}

#[cfg(windows)]
pub use windows::spawn_session_watcher;

#[cfg(target_os = "linux")]
pub use linux::spawn_session_watcher;

#[cfg(not(any(windows, target_os = "linux")))]
pub fn spawn_session_watcher(_sink: EventSink) {}

pub const fn platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "linux") {
        Platform::Linux
    } else {
        Platform::Other
    }
}

pub fn native_backdrop_supported() -> Backdrop {
    #[cfg(windows)]
    if windows::supports_acrylic() {
        return Backdrop::Acrylic;
    }
    Backdrop::None
}

pub fn is_wayland_session() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var("XDG_SESSION_TYPE").is_ok_and(|t| t.eq_ignore_ascii_case("wayland"))
            || std::env::var_os("WAYLAND_DISPLAY").is_some())
}

#[cfg(not(any(windows, target_os = "linux")))]
mod unsupported {
    use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
    use crate::models::{Capability, SignInSurface};
    use crate::session::{ProviderCapabilities, SessionKey, SessionProvider, SystemSnapshot};

    pub struct UnsupportedProvider;

    impl SessionProvider for UnsupportedProvider {
        fn snapshot(&self) -> SwitcherResult<SystemSnapshot> {
            Err(SwitcherError::new(ErrorCode::UnsupportedPlatform))
        }

        fn capabilities(&self) -> ProviderCapabilities {
            let none = Capability::unavailable(ErrorCode::UnsupportedPlatform);
            ProviderCapabilities {
                session_discovery: none,
                account_directory: none,
                system_sign_in: none,
                sign_in_surface: SignInSurface::None,
                same_user_session_switch: none,
                lock_screen: none,
            }
        }

        fn activate_own_session(&self, _: &SessionKey) -> SwitcherResult<()> {
            Err(SwitcherError::new(ErrorCode::UnsupportedPlatform))
        }

        fn open_system_sign_in(&self) -> SwitcherResult<()> {
            Err(SwitcherError::new(ErrorCode::UnsupportedPlatform))
        }

        fn lock_current_session(&self) -> SwitcherResult<()> {
            Err(SwitcherError::new(ErrorCode::UnsupportedPlatform))
        }
    }
}
