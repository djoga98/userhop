use std::ffi::{CStr, c_char, c_int};
use std::os::unix::fs::MetadataExt;
use std::sync::{Mutex, OnceLock};

use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::platform::macos_model::{RawLogin, RawMacUser, fast_user_switching_enabled};

const MAX_USERS: usize = 1024;
const MAX_LOGINS: usize = 512;
const LOGIN_FRAMEWORK: &CStr =
    c"/System/Library/PrivateFrameworks/login.framework/Versions/Current/login";
const GLOBAL_PREFERENCES: &str = "/Library/Preferences/.GlobalPreferences.plist";
const MAX_PREFERENCES_BYTES: u64 = 4 * 1024 * 1024;

/// `getpwent`, `getpwuid` and `getutxent` share process-wide static state.
static DIRECTORY_LOCK: Mutex<()> = Mutex::new(());

type LoginCall = unsafe extern "C" fn() -> c_int;

#[derive(Clone, Copy)]
pub struct LoginFramework {
    pub switch_to_login_window: Option<LoginCall>,
    pub lock_screen: Option<LoginCall>,
}

/// The same system framework the Fast User Switching menu uses. It is not public API, so every
/// entry point is resolved at runtime and a missing symbol disables the feature instead of failing.
pub fn login_framework() -> Option<LoginFramework> {
    static FRAMEWORK: OnceLock<Option<LoginFramework>> = OnceLock::new();
    *FRAMEWORK.get_or_init(|| {
        let handle =
            unsafe { libc::dlopen(LOGIN_FRAMEWORK.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
        if handle.is_null() {
            tracing::warn!("login framework unavailable");
            return None;
        }
        let symbol = |name: &CStr| -> Option<LoginCall> {
            let pointer = unsafe { libc::dlsym(handle, name.as_ptr()) };
            (!pointer.is_null())
                .then(|| unsafe { std::mem::transmute::<*mut libc::c_void, LoginCall>(pointer) })
        };
        Some(LoginFramework {
            switch_to_login_window: symbol(c"SACSwitchToLoginWindow"),
            lock_screen: symbol(c"SACLockScreenImmediate"),
        })
    })
}

fn call(entry: Option<LoginCall>, api: &'static str) -> SwitcherResult<()> {
    let entry = entry.ok_or_else(|| SwitcherError::new(ErrorCode::UnsupportedDesktop))?;
    match unsafe { entry() } {
        0 => Ok(()),
        status => Err(SwitcherError::os(api, status)),
    }
}

pub fn switch_to_login_window() -> SwitcherResult<()> {
    call(
        login_framework().and_then(|l| l.switch_to_login_window),
        "SACSwitchToLoginWindow",
    )
}

pub fn lock_screen() -> SwitcherResult<()> {
    call(
        login_framework().and_then(|l| l.lock_screen),
        "SACLockScreenImmediate",
    )
}

pub fn own_uid() -> u32 {
    rustix::process::getuid().as_raw()
}

/// The owner of /dev/console is the user whose session is on screen.
pub fn console_uid() -> Option<u32> {
    std::fs::metadata("/dev/console")
        .ok()
        .map(|m| m.uid())
        .filter(|&uid| uid != 0)
}

pub fn fast_user_switching_preference() -> Option<bool> {
    let metadata = std::fs::metadata(GLOBAL_PREFERENCES).ok()?;
    if metadata.len() > MAX_PREFERENCES_BYTES {
        return None;
    }
    let value = plist::Value::from_file(GLOBAL_PREFERENCES).ok()?;
    fast_user_switching_enabled(&value)
}

fn text(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();
    (!value.is_empty()).then_some(value)
}

fn fixed_text(buffer: &[c_char]) -> String {
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

unsafe fn user_from(entry: &libc::passwd) -> Option<RawMacUser> {
    Some(RawMacUser {
        uid: entry.pw_uid,
        name: text(entry.pw_name)?,
        full_name: text(entry.pw_gecos),
        shell: text(entry.pw_shell).unwrap_or_default(),
    })
}

pub fn user_by_uid(uid: u32) -> Option<RawMacUser> {
    let _guard = DIRECTORY_LOCK.lock().ok()?;
    let entry = unsafe { libc::getpwuid(uid) };
    if entry.is_null() {
        return None;
    }
    unsafe { user_from(&*entry) }
}

fn uid_by_name(name: &str) -> Option<u32> {
    let name = std::ffi::CString::new(name).ok()?;
    let entry = unsafe { libc::getpwnam(name.as_ptr()) };
    (!entry.is_null()).then(|| unsafe { (*entry).pw_uid })
}

pub fn local_users() -> Option<Vec<RawMacUser>> {
    let _guard = DIRECTORY_LOCK.lock().ok()?;
    let mut users = Vec::new();
    unsafe {
        libc::setpwent();
        while users.len() < MAX_USERS {
            let entry = libc::getpwent();
            if entry.is_null() {
                break;
            }
            if let Some(user) = user_from(&*entry) {
                users.push(user);
            }
        }
        libc::endpwent();
    }
    Some(users)
}

pub fn logins() -> Vec<RawLogin> {
    let Ok(_guard) = DIRECTORY_LOCK.lock() else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    unsafe {
        libc::setutxent();
        while entries.len() < MAX_LOGINS {
            let entry = libc::getutxent();
            if entry.is_null() {
                break;
            }
            let entry = &*entry;
            if entry.ut_type == libc::USER_PROCESS {
                entries.push((
                    fixed_text(&entry.ut_user),
                    fixed_text(&entry.ut_line),
                    fixed_text(&entry.ut_host),
                    entry.ut_pid,
                ));
            }
        }
        libc::endutxent();
    }
    entries
        .into_iter()
        .filter_map(|(name, line, host, pid)| {
            Some(RawLogin {
                uid: uid_by_name(&name)?,
                name,
                line,
                host,
                pid,
            })
        })
        .collect()
}
