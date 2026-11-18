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
