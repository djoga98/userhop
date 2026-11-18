use std::ptr::NonNull;

use block2::RcBlock;
use objc2_app_kit::{
    NSWorkspace, NSWorkspaceDidWakeNotification, NSWorkspaceScreensDidSleepNotification,
    NSWorkspaceSessionDidBecomeActiveNotification, NSWorkspaceSessionDidResignActiveNotification,
};
use objc2_foundation::{
    NSDistributedNotificationCenter, NSNotification, NSNotificationCenter, NSOperationQueue,
    NSString,
};

use crate::platform::{EventSink, SessionEvent};

fn observe(center: &NSNotificationCenter, name: &NSString, sink: &EventSink, event: SessionEvent) {
    let sink = sink.clone();
    let block = RcBlock::new(move |_: NonNull<NSNotification>| sink(event));
    let queue = NSOperationQueue::mainQueue();
    let token = unsafe {
        center.addObserverForName_object_queue_usingBlock(Some(name), None, Some(&queue), &block)
    };
    // Observers live for the whole process; the token is the only handle that would remove them.
    std::mem::forget(token);
}

/// Fast User Switching away, display sleep and screen lock hide the flyout; returning refreshes it.
pub fn spawn_session_watcher(sink: EventSink) {
    let workspace = NSWorkspace::sharedWorkspace().notificationCenter();
    let names = unsafe {
        [
            (
                NSWorkspaceSessionDidResignActiveNotification,
                SessionEvent::CurrentLocked,
            ),
            (
                NSWorkspaceScreensDidSleepNotification,
                SessionEvent::CurrentLocked,
            ),
            (
                NSWorkspaceSessionDidBecomeActiveNotification,
                SessionEvent::Changed,
            ),
            (NSWorkspaceDidWakeNotification, SessionEvent::Resumed),
        ]
    };
    for (name, event) in names {
        observe(&workspace, name, &sink, event);
    }
    let distributed = NSDistributedNotificationCenter::defaultCenter();
    observe(
        &distributed,
        &NSString::from_str("com.apple.screenIsLocked"),
        &sink,
        SessionEvent::CurrentLocked,
    );
    observe(
        &distributed,
        &NSString::from_str("com.apple.screenIsUnlocked"),
        &sink,
        SessionEvent::Changed,
    );
}
