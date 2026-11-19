use std::sync::OnceLock;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_ALL_SESSIONS, NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, PBT_APMRESUMEAUTOMATIC,
    RegisterClassW, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_POWERBROADCAST,
    WM_WTSSESSION_CHANGE, WNDCLASSW, WTS_CONSOLE_DISCONNECT, WTS_REMOTE_DISCONNECT,
    WTS_SESSION_LOCK,
};
use windows::core::w;

use crate::platform::{EventSink, SessionEvent};

static SINK: OnceLock<(EventSink, u32)> = OnceLock::new();

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if let Some((sink, own_session)) = SINK.get() {
        match msg {
            WM_WTSSESSION_CHANGE => {
                let event = wparam.0 as u32;
                let session = lparam.0 as u32;
                let hides_own_desktop = matches!(
                    event,
                    WTS_SESSION_LOCK | WTS_CONSOLE_DISCONNECT | WTS_REMOTE_DISCONNECT
                );
                if session == *own_session && hides_own_desktop {
                    sink(SessionEvent::CurrentLocked);
                } else {
                    sink(SessionEvent::Changed);
                }
            }
            WM_POWERBROADCAST if wparam.0 as u32 == PBT_APMRESUMEAUTOMATIC => {
                sink(SessionEvent::Resumed);
            }
            _ => {}
        }
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// A hidden top-level window receives both WTS session and power broadcast messages.
pub fn spawn_session_watcher(sink: EventSink) {
    let Ok(own_session) = super::native::own_session_id() else {
        tracing::warn!("session watcher disabled: no session id");
        return;
    };
    if SINK.set((sink, own_session)).is_err() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("session-watcher".into())
        .spawn(|| {
            if let Err(error) = run_message_loop() {
                tracing::warn!(%error, "session watcher stopped");
            }
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "session watcher thread failed");
    }
}

fn run_message_loop() -> windows::core::Result<()> {
    let module = unsafe { GetModuleHandleW(None) }?;
    let class_name = w!("UserhopSessionWatcher");
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: module.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(windows::core::Error::from_thread());
    }
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(module.into()),
            None,
        )
    }?;
    if unsafe { WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_ALL_SESSIONS) }.is_err() {
        unsafe { WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) }?;
    }
    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}
