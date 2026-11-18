use std::ffi::c_void;
use std::ptr;

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_MORE_DATA, HANDLE, HLOCAL, LocalFree,
};
use windows::Win32::NetworkManagement::NetManagement::{
    FILTER_NORMAL_ACCOUNT, MAX_PREFERRED_LENGTH, NERR_Success, NetApiBufferFree, NetUserEnum,
    NetUserGetInfo, UF_ACCOUNTDISABLE, USER_INFO_20, USER_INFO_23,
};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{
    GetTokenInformation, LookupAccountNameW, PSID, SID_NAME_USE, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows::Win32::System::RemoteDesktop::{
    ProcessIdToSessionId, WTS_CONNECTSTATE_CLASS, WTS_INFO_CLASS, WTS_SESSION_INFOW, WTSActive,
    WTSClientProtocolType, WTSConnected, WTSDisconnected, WTSDomainName, WTSEnumerateSessionsW,
    WTSFreeMemory, WTSGetActiveConsoleSessionId, WTSQuerySessionInformationW, WTSUserName,
};
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, OpenProcessToken};
use windows::core::{HSTRING, PCWSTR, PWSTR, w};

use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::platform::windows_model::{RawLocalAccount, RawSession, WtsState};

const MAX_SESSIONS: usize = 512;
const MAX_ACCOUNTS: usize = 512;
const MAX_STRING_UNITS: usize = 512;

struct WtsMemory(*mut c_void);

impl Drop for WtsMemory {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { WTSFreeMemory(self.0) };
        }
    }
}

struct NetBuffer(*mut u8);

impl Drop for NetBuffer {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { NetApiBufferFree(Some(self.0.cast_const().cast())) };
        }
    }
}

struct LocalString(PWSTR);

impl Drop for LocalString {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { LocalFree(Some(HLOCAL(self.0.0.cast()))) };
        }
    }
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

/// Reads a NUL-terminated UTF-16 string with an explicit upper bound.
fn read_wide(value: PWSTR) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let mut len = 0usize;
    while len < MAX_STRING_UNITS && unsafe { *value.0.add(len) } != 0 {
        len += 1;
    }
    let units = unsafe { std::slice::from_raw_parts(value.0, len) };
    String::from_utf16(units).ok().filter(|s| !s.is_empty())
}

fn map_error(api: &'static str, error: &windows::core::Error) -> SwitcherError {
    if error.code() == ERROR_ACCESS_DENIED.to_hresult() {
        SwitcherError::with_log(ErrorCode::PermissionDenied, api, error)
    } else {
        SwitcherError::os(api, error)
    }
}

pub fn own_session_id() -> SwitcherResult<u32> {
    let mut session = 0u32;
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) }
        .map_err(|e| map_error("ProcessIdToSessionId", &e))?;
    Ok(session)
}

pub fn active_console_session_id() -> Option<u32> {
    let id = unsafe { WTSGetActiveConsoleSessionId() };
    (id != u32::MAX).then_some(id)
}

fn wts_state(state: WTS_CONNECTSTATE_CLASS) -> WtsState {
    match state {
        s if s == WTSActive => WtsState::Active,
        s if s == WTSConnected => WtsState::Connected,
        s if s == WTSDisconnected => WtsState::Disconnected,
        _ => WtsState::Other,
    }
}

fn query_raw(session: u32, class: WTS_INFO_CLASS) -> Option<(WtsMemory, u32)> {
    let mut buffer = PWSTR::null();
    let mut bytes = 0u32;
    unsafe { WTSQuerySessionInformationW(None, session, class, &mut buffer, &mut bytes) }.ok()?;
    Some((WtsMemory(buffer.0.cast()), bytes))
}

fn query_string(session: u32, class: WTS_INFO_CLASS) -> Option<String> {
    let (memory, _) = query_raw(session, class)?;
    read_wide(PWSTR(memory.0.cast()))
}

pub fn query_protocol(session: u32) -> Option<u16> {
    let (memory, bytes) = query_raw(session, WTSClientProtocolType)?;
    if memory.0.is_null() || (bytes as usize) < size_of::<u16>() {
        return None;
    }
    Some(unsafe { memory.0.cast::<u16>().read_unaligned() })
}

pub fn enumerate_sessions() -> SwitcherResult<Vec<RawSession>> {
    let mut info: *mut WTS_SESSION_INFOW = ptr::null_mut();
    let mut count = 0u32;
    unsafe { WTSEnumerateSessionsW(None, 0, 1, &mut info, &mut count) }
        .map_err(|e| map_error("WTSEnumerateSessionsW", &e))?;
    let memory = WtsMemory(info.cast());
    if memory.0.is_null() || count == 0 {
        return Ok(Vec::new());
    }
    let entries = unsafe { std::slice::from_raw_parts(info, (count as usize).min(MAX_SESSIONS)) };
    Ok(entries
        .iter()
        .map(|entry| {
            let user_name = query_string(entry.SessionId, WTSUserName);
            let domain = query_string(entry.SessionId, WTSDomainName);
            let sid = user_name
                .as_deref()
                .and_then(|user| lookup_account_sid(domain.as_deref(), user));
            RawSession {
                id: entry.SessionId,
                state: wts_state(entry.State),
                user_name,
                sid,
                protocol: query_protocol(entry.SessionId),
            }
        })
        .collect())
}

fn sid_to_string(sid: PSID) -> Option<String> {
    if sid.0.is_null() {
        return None;
    }
    let mut out = PWSTR::null();
    unsafe { ConvertSidToStringSidW(sid, &mut out) }.ok()?;
    let owned = LocalString(out);
    read_wide(owned.0)
}

pub fn current_user_sid() -> SwitcherResult<String> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
        .map_err(|e| map_error("OpenProcessToken", &e))?;
    let token = OwnedHandle(token);
    let mut needed = 0u32;
    let _ = unsafe { GetTokenInformation(token.0, TokenUser, None, 0, &mut needed) };
    if needed == 0 || needed > 4096 {
        return Err(SwitcherError::new(ErrorCode::OsApiFailure));
    }
    let mut buffer = vec![0u64; (needed as usize).div_ceil(size_of::<u64>())];
    unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )
    }
    .map_err(|e| map_error("GetTokenInformation", &e))?;
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    sid_to_string(user.User.Sid).ok_or_else(|| SwitcherError::new(ErrorCode::OsApiFailure))
}

fn lookup_account_sid(domain: Option<&str>, user: &str) -> Option<String> {
    let qualified = match domain {
        Some(domain) => HSTRING::from(format!("{domain}\\{user}")),
        None => HSTRING::from(user),
    };
    let mut sid_len = 0u32;
    let mut domain_len = 0u32;
    let mut usage = SID_NAME_USE::default();
    let _ = unsafe {
        LookupAccountNameW(
            PCWSTR::null(),
            &qualified,
            None,
            &mut sid_len,
            None,
            &mut domain_len,
            &mut usage,
        )
    };
    if sid_len == 0 || sid_len > 256 || domain_len > 1024 {
        return None;
    }
    let mut sid = vec![0u32; (sid_len as usize).div_ceil(size_of::<u32>())];
    let mut domain_buffer = vec![0u16; domain_len as usize + 1];
    unsafe {
        LookupAccountNameW(
            PCWSTR::null(),
            &qualified,
            Some(PSID(sid.as_mut_ptr().cast())),
            &mut sid_len,
            Some(PWSTR(domain_buffer.as_mut_ptr())),
            &mut domain_len,
            &mut usage,
        )
    }
    .ok()?;
    sid_to_string(PSID(sid.as_mut_ptr().cast()))
}

struct UserDetails {
    full_name: Option<String>,
    sid: Option<String>,
}

fn user_details(name: &str) -> Option<UserDetails> {
    let mut buffer: *mut u8 = ptr::null_mut();
    let status = unsafe { NetUserGetInfo(PCWSTR::null(), &HSTRING::from(name), 23, &mut buffer) };
    let owned = NetBuffer(buffer);
    if status != NERR_Success || owned.0.is_null() {
        return None;
    }
    let info = unsafe { &*owned.0.cast::<USER_INFO_23>() };
    Some(UserDetails {
        full_name: read_wide(info.usri23_full_name),
        sid: sid_to_string(info.usri23_user_sid),
    })
}

pub fn current_user_full_name(user_name: &str) -> Option<String> {
    user_details(user_name).and_then(|d| d.full_name)
}

fn is_hidden_from_sign_in(name: &str) -> bool {
    reg_dword(
        HKEY_LOCAL_MACHINE,
        w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon\\SpecialAccounts\\UserList"),
        &HSTRING::from(name),
    ) == Some(0)
}

pub fn local_accounts() -> Option<Vec<RawLocalAccount>> {
    let mut buffer: *mut u8 = ptr::null_mut();
    let mut read = 0u32;
    let mut total = 0u32;
    let status = unsafe {
        NetUserEnum(
            PCWSTR::null(),
            20,
            FILTER_NORMAL_ACCOUNT,
            &mut buffer,
            MAX_PREFERRED_LENGTH,
            &mut read,
            &mut total,
            None,
        )
    };
    let owned = NetBuffer(buffer);
    if (status != NERR_Success && status != ERROR_MORE_DATA.0) || owned.0.is_null() {
        tracing::warn!(status, "NetUserEnum failed");
        return None;
    }
    let users = unsafe {
        std::slice::from_raw_parts(
            owned.0.cast::<USER_INFO_20>(),
            (read as usize).min(MAX_ACCOUNTS),
        )
    };
    Some(
        users
            .iter()
            .filter_map(|user| {
                let name = read_wide(user.usri20_name)?;
                let details = user_details(&name);
                Some(RawLocalAccount {
                    full_name: read_wide(user.usri20_full_name),
                    sid: details.and_then(|d| d.sid),
                    disabled: user.usri20_flags.0 & UF_ACCOUNTDISABLE.0 != 0,
                    hidden: is_hidden_from_sign_in(&name),
                    name,
                })
            })
            .collect(),
    )
}

fn reg_dword(root: HKEY, subkey: PCWSTR, value: &HSTRING) -> Option<u32> {
    let mut data = 0u32;
    let mut size = size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            root,
            subkey,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut data).cast()),
            Some(&mut size),
        )
    };
    status.is_ok().then_some(data)
}

const POLICIES_SYSTEM: PCWSTR =
    w!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System");

pub fn hide_fast_user_switching() -> bool {
    reg_dword(
        HKEY_LOCAL_MACHINE,
        POLICIES_SYSTEM,
        &HSTRING::from("HideFastUserSwitching"),
    ) == Some(1)
}

pub fn lock_workstation_disabled() -> bool {
    let name = HSTRING::from("DisableLockWorkstation");
    reg_dword(HKEY_CURRENT_USER, POLICIES_SYSTEM, &name) == Some(1)
        || reg_dword(HKEY_LOCAL_MACHINE, POLICIES_SYSTEM, &name) == Some(1)
}

pub fn windows_build_number() -> Option<u32> {
    let mut data = [0u16; 32];
    let mut size = (data.len() * size_of::<u16>()) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(data.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status.is_err() {
        return None;
    }
    let len = data.iter().position(|&c| c == 0).unwrap_or(data.len());
    String::from_utf16(&data[..len]).ok()?.parse().ok()
}

pub fn lock_workstation() -> SwitcherResult<()> {
    unsafe { LockWorkStation() }.map_err(|e| map_error("LockWorkStation", &e))
}
