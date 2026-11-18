use zbus::proxy;
use zbus::zvariant::OwnedObjectPath;

pub type SessionListing = (String, u32, String, String, OwnedObjectPath);

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1",
    gen_async = false
)]
pub trait Manager {
    fn list_sessions(&self) -> zbus::Result<Vec<SessionListing>>;

    fn get_session(&self, session_id: &str) -> zbus::Result<OwnedObjectPath>;

    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, pid: u32) -> zbus::Result<OwnedObjectPath>;

    fn get_user(&self, uid: u32) -> zbus::Result<OwnedObjectPath>;

    fn activate_session_on_seat(&self, session_id: &str, seat_id: &str) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    gen_async = false
)]
pub trait Session {
    fn activate(&self) -> zbus::Result<()>;

    fn lock(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn user(&self) -> zbus::Result<(u32, OwnedObjectPath)>;

    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn seat(&self) -> zbus::Result<(String, OwnedObjectPath)>;

    #[zbus(property)]
    fn class(&self) -> zbus::Result<String>;

    #[zbus(property, name = "Type")]
    fn session_type(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn remote(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn locked_hint(&self) -> zbus::Result<bool>;
}

#[proxy(
    interface = "org.freedesktop.login1.Seat",
    default_service = "org.freedesktop.login1",
    gen_async = false
)]
pub trait Seat {
    #[zbus(property)]
    fn can_multi_session(&self) -> zbus::Result<bool>;
}

#[proxy(
    interface = "org.freedesktop.login1.User",
    default_service = "org.freedesktop.login1",
    gen_async = false
)]
pub trait User {
    #[zbus(property)]
    fn display(&self) -> zbus::Result<(String, OwnedObjectPath)>;

    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;
}

#[proxy(
    interface = "org.freedesktop.Accounts",
    default_service = "org.freedesktop.Accounts",
    default_path = "/org/freedesktop/Accounts",
    gen_async = false
)]
pub trait Accounts {
    fn list_cached_users(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[proxy(
    interface = "org.freedesktop.Accounts.User",
    default_service = "org.freedesktop.Accounts",
    gen_async = false
)]
pub trait AccountsUser {
    #[zbus(property)]
    fn uid(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn user_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn real_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn system_account(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn locked(&self) -> zbus::Result<bool>;
}

#[proxy(
    interface = "org.gnome.DisplayManager.LocalDisplayFactory",
    default_service = "org.gnome.DisplayManager",
    default_path = "/org/gnome/DisplayManager/LocalDisplayFactory",
    gen_async = false
)]
pub trait GdmLocalDisplayFactory {
    fn create_transient_display(&self) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.DisplayManager",
    default_service = "org.freedesktop.DisplayManager",
    default_path = "/org/freedesktop/DisplayManager",
    gen_async = false
)]
pub trait DisplayManager {
    #[zbus(property)]
    fn seats(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[proxy(
    interface = "org.freedesktop.DisplayManager.Seat",
    default_service = "org.freedesktop.DisplayManager",
    gen_async = false
)]
pub trait DisplayManagerSeat {
    fn switch_to_greeter(&self) -> zbus::Result<()>;
}
