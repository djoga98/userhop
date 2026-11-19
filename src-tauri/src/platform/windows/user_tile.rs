use std::ffi::c_void;
use std::ptr::null_mut;

use windows::Win32::System::WinRT::{
    RO_INIT_MULTITHREADED, RoGetActivationFactory, RoInitialize, RoUninitialize,
};
use windows::core::{GUID, HRESULT, HSTRING, IInspectable_Vtbl, IUnknown, Interface, Type};

use crate::errors::SwitcherResult;
use crate::session::AccountKey;

use super::native::map_error;

const USER_TILE_MENU: &str = "Windows.Internal.Shell.StartUI.UserTileMenu";

// The interface IDs and relevant vtable slots match the shell broker binaries from
// Windows 10 22H2 (19041.6216) and Windows 11 (26100.9818).
#[repr(transparent)]
#[derive(Clone)]
struct IUserTileMenuStatics(IUnknown);

unsafe impl Interface for IUserTileMenuStatics {
    type Vtable = UserTileMenuStaticsVtbl;
    const IID: GUID = GUID::from_u128(0x1727f7b0_3bd5_4256_8e75_680f9f6cadb4);
}

#[repr(C)]
struct UserTileMenuStaticsVtbl {
    base: IInspectable_Vtbl,
    get_commands: usize,
    get_switch_user_list: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(transparent)]
#[derive(Clone)]
struct IUserTileSwitchUserVectorView(IUnknown);

unsafe impl Interface for IUserTileSwitchUserVectorView {
    type Vtable = UserTileSwitchUserVectorViewVtbl;
    const IID: GUID = GUID::from_u128(0x9460aafe_41e0_54c6_8c1b_24abfceae321);
}

#[repr(C)]
struct UserTileSwitchUserVectorViewVtbl {
    base: IInspectable_Vtbl,
    get_at: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> HRESULT,
    get_size: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
}

#[repr(transparent)]
#[derive(Clone)]
struct IUserTileSwitchUser(IUnknown);

unsafe impl Interface for IUserTileSwitchUser {
    type Vtable = UserTileSwitchUserVtbl;
    const IID: GUID = GUID::from_u128(0x94432454_c6ac_40c0_ba40_d5c31a0c5136);
}

#[repr(C)]
struct UserTileSwitchUserVtbl {
    base: IInspectable_Vtbl,
    get_display_name: usize,
    get_user_sid: unsafe extern "system" fn(*mut c_void, *mut HSTRING) -> HRESULT,
    get_is_logged_in: usize,
    get_account_picture: usize,
    switch_to: unsafe extern "system" fn(*mut c_void) -> HRESULT,
}

struct WinRtApartment;

impl WinRtApartment {
    fn initialize() -> windows::core::Result<Self> {
        // The IPC command and tray action both use a blocking worker thread.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
        Ok(Self)
    }
}

impl Drop for WinRtApartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

fn find_user(
    _apartment: &WinRtApartment,
    target_sid: &str,
) -> windows::core::Result<Option<IUserTileSwitchUser>> {
    // The factory is obtained through WinRT. The list and its entries use the
    // WinRT ABI and transfer ownership of each returned interface and string.
    unsafe {
        let menu: IUserTileMenuStatics = RoGetActivationFactory(&HSTRING::from(USER_TILE_MENU))?;
        let mut list = null_mut();
        (menu.vtable().get_switch_user_list)(menu.as_raw(), &mut list).ok()?;
        let list = IUserTileSwitchUserVectorView::from_abi(list)?;
        let mut count = 0;
        (list.vtable().get_size)(list.as_raw(), &mut count).ok()?;

        for index in 0..count {
            let mut user = null_mut();
            (list.vtable().get_at)(list.as_raw(), index, &mut user).ok()?;
            let user = IUserTileSwitchUser::from_abi(user)?;
            let mut sid = HSTRING::new();
            (user.vtable().get_user_sid)(user.as_raw(), &mut sid).ok()?;
            if sid == target_sid {
                return Ok(Some(user));
            }
        }
        Ok(None)
    }
}

/// Returns false if the account-specific handoff could not be prepared.
pub(super) fn try_switch_to(account: &AccountKey) -> SwitcherResult<bool> {
    let Some(sid) = account
        .0
        .strip_prefix("win:")
        .filter(|sid| sid.starts_with("S-1-"))
    else {
        return Ok(false);
    };
    let apartment = match WinRtApartment::initialize() {
        Ok(apartment) => apartment,
        Err(error) => {
            tracing::debug!(code = ?error.code(), "could not initialize WinRT for account switching");
            return Ok(false);
        }
    };
    let user = match find_user(&apartment, sid) {
        Ok(Some(user)) => user,
        Ok(None) => return Ok(false),
        Err(error) => {
            tracing::debug!(code = ?error.code(), "Windows account switch target unavailable");
            return Ok(false);
        }
    };

    // Once the broker is invoked, report its result without initiating another handoff.
    unsafe { (user.vtable().switch_to)(user.as_raw()).ok() }
        .map_err(|error| map_error("UserTileSwitchUser.SwitchTo", &error))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::windows::WindowsProvider;
    use crate::session::SessionProvider;

    #[test]
    #[ignore = "requires an interactive Windows session with another sign-in account"]
    fn native_user_tile_menu_matches_an_account_sid() {
        let snapshot = WindowsProvider
            .snapshot()
            .expect("Windows account snapshot");
        let apartment = WinRtApartment::initialize().expect("WinRT apartment");
        let found = snapshot
            .accounts
            .iter()
            .filter(|account| account.key != snapshot.current_account)
            .filter_map(|account| account.key.0.strip_prefix("win:"))
            .any(|sid| {
                find_user(&apartment, sid)
                    .expect("Windows sign-in accounts")
                    .is_some()
            });
        assert!(
            found,
            "a Windows sign-in entry must match another account by SID"
        );
    }
}
