use std::collections::HashMap;

use zbus::MatchRule;
use zbus::blocking::{Connection, MessageIterator};
use zbus::message::Type;
use zbus::zvariant::OwnedValue;

use crate::platform::{EventSink, SessionEvent};

use super::LinuxProvider;

type PropertiesChanged = (String, HashMap<String, OwnedValue>, Vec<String>);

fn classify(message: &zbus::Message, own_path: Option<&str>) -> Option<SessionEvent> {
    let header = message.header();
    let member = header.member()?.as_str();
    let path = header.path().map(|p| p.as_str());
    let on_own_session = own_path.is_some() && path == own_path;
    match member {
        "SessionNew" | "SessionRemoved" | "UserNew" | "UserRemoved" => Some(SessionEvent::Changed),
        "PrepareForSleep" => match message.body().deserialize::<bool>() {
            Ok(false) => Some(SessionEvent::Resumed),
            _ => None,
        },
        "Lock" if on_own_session => Some(SessionEvent::CurrentLocked),
        "PropertiesChanged" if on_own_session => {
            let (_, changed, _) = message.body().deserialize::<PropertiesChanged>().ok()?;
            let flag = |name: &str| changed.get(name).and_then(|v| bool::try_from(v).ok());
            if flag("LockedHint") == Some(true) || flag("Active") == Some(false) {
                Some(SessionEvent::CurrentLocked)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn spawn_session_watcher(sink: EventSink) {
    let spawned = std::thread::Builder::new()
        .name("logind-watcher".into())
        .spawn(move || {
            if let Err(error) = watch(&sink) {
                tracing::warn!(%error, "logind watcher stopped");
            }
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "logind watcher thread failed");
    }
}

fn watch(sink: &EventSink) -> zbus::Result<()> {
    let own_path = LinuxProvider::new()
        .own_session_path()
        .map(|(_, path)| path.as_str().to_owned());
    let connection = Connection::system()?;
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender("org.freedesktop.login1")?
        .build();
    for message in MessageIterator::for_match_rule(rule, &connection, Some(64))? {
        if let Some(event) = classify(&message?, own_path.as_deref()) {
            sink(event);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use zbus::Message;
    use zbus::zvariant::Value;

    use super::*;

    const OWN: &str = "/org/freedesktop/login1/session/_32";
    const OTHER: &str = "/org/freedesktop/login1/session/_35";
    const SESSION_IFACE: &str = "org.freedesktop.login1.Session";
    const PROPS_IFACE: &str = "org.freedesktop.DBus.Properties";

    fn properties(path: &str, name: &str, value: bool) -> Message {
        let changed = HashMap::from([(name.to_owned(), Value::from(value))]);
        Message::signal(path, PROPS_IFACE, "PropertiesChanged")
            .expect("builder")
            .build(&(SESSION_IFACE, changed, Vec::<String>::new()))
            .expect("message")
    }

    #[test]
    fn own_lock_signals_hide_the_flyout() {
        let lock = Message::signal(OWN, SESSION_IFACE, "Lock")
            .expect("builder")
            .build(&())
            .expect("message");
        assert_eq!(
            classify(&lock, Some(OWN)),
            Some(SessionEvent::CurrentLocked)
        );
        assert_eq!(
            classify(&properties(OWN, "LockedHint", true), Some(OWN)),
            Some(SessionEvent::CurrentLocked)
        );
        assert_eq!(
            classify(&properties(OWN, "Active", false), Some(OWN)),
            Some(SessionEvent::CurrentLocked)
        );
    }

    #[test]
    fn other_sessions_and_unlocks_do_not_hide() {
        assert_eq!(
            classify(&properties(OTHER, "LockedHint", true), Some(OWN)),
            None
        );
        assert_eq!(
            classify(&properties(OWN, "LockedHint", false), Some(OWN)),
            None
        );
        let lock_other = Message::signal(OTHER, SESSION_IFACE, "Lock")
            .expect("builder")
            .build(&())
            .expect("message");
        assert_eq!(classify(&lock_other, Some(OWN)), None);
        assert_eq!(classify(&lock_other, None), None);
    }

    #[test]
    fn session_churn_and_resume_refresh_the_list() {
        let manager = "/org/freedesktop/login1";
        let iface = "org.freedesktop.login1.Manager";
        let new = Message::signal(manager, iface, "SessionNew")
            .expect("builder")
            .build(&(
                "c3",
                zbus::zvariant::ObjectPath::try_from(OTHER).expect("path"),
            ))
            .expect("message");
        assert_eq!(classify(&new, Some(OWN)), Some(SessionEvent::Changed));
        let sleeping = Message::signal(manager, iface, "PrepareForSleep")
            .expect("builder")
            .build(&true)
            .expect("message");
        assert_eq!(classify(&sleeping, Some(OWN)), None);
        let resumed = Message::signal(manager, iface, "PrepareForSleep")
            .expect("builder")
            .build(&false)
            .expect("message");
        assert_eq!(classify(&resumed, Some(OWN)), Some(SessionEvent::Resumed));
    }
}
