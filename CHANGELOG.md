# Changelog

## 0.1.0 — unreleased (2026-11-18)

**Verified targets:** Windows 11 Pro build 26300 x64 (read-only session enumeration test, desktop smoke test of the debug and release builds). Linux build and logind enumeration verified on Ubuntu 24.04 under WSL2 only; no real GNOME/KDE desktop tested yet.

**Features**

- macOS support (preview): accounts from the local directory and console sessions from `utmpx`, switching through the macOS login window, lock screen, menu-bar-only app with a template icon, native vibrancy, per-user single instance, launch at sign-in via a LaunchAgent, universal `.dmg` (Apple Silicon and Intel). Default shortcut on macOS is ⌃⌥U because ⌘⇧U belongs to Finder.

- Windows 10 and 11 support. The flyout is styled after the Windows 11 clipboard panel, with native Acrylic on Windows 11 and a solid Fluent-tinted fallback on Windows 10 and Linux; light, dark and system themes; high-contrast (`forced-colors`) styling; reduced motion.
- Real account and session discovery: Windows WTS sessions plus local accounts from NetAPI (disabled and sign-in-hidden accounts excluded); Linux logind sessions plus AccountsService users.
- Switching hands off to the OS: on Windows the PC is locked with `LockWorkStation` and the person picks the account on the Windows sign-in screen (the session stays attached to its displays); Linux login screen (GDM, LightDM, SDDM) after a confirmed lock of the own session; macOS login window; direct activation only for the user's own second session.
- Lock screen action (tray, footer).
- Switch back to the last account: the last account you switched to is marked *Last used*, listed first and focused when the flyout opens (shortcut + Enter), and offered in the tray as *Switch back to …*. It is kept in memory only, so nothing about other users is written to disk; the request goes through the same fresh validation as any other switch.
- Global shortcut (default Win/Super + Shift + U) with conflict detection and in-app capture; `--toggle` for desktops without global shortcuts.
- Launch at sign-in (per user, off by default).
- Keyboard: arrows/Home/End move between rows, Enter/Space activate, Escape goes back then closes, focus moves into Settings and back.

**Fixes after the first test build**

- Windows: switching no longer disconnects the console session (`WTSDisconnectSession`), which looked like a plain lock and rearranged desktop icons on multi-monitor setups when returning. It now locks the PC like Win+L, and the panel explains that the account is chosen in the bottom-left corner of the sign-in screen.
- The flyout stays hidden until the new state is rendered and sized, so it no longer flickers on open; the CSS fade is skipped where the OS draws Acrylic or vibrancy.
- Settings opens at its full height (up to 700 px within the work area); previously the size tracking stopped after the first reopen.
- Tab and Shift+Tab now cycle inside the panel instead of leaving it.

**Security behavior**

OS authentication remains native. The app never accepts or stores credentials, exposes twelve narrow commands guarded by Tauri app permissions, revalidates every switch request against a fresh OS snapshot, allows one switch request at a time and never retries automatically. Strict CSP, no remote content, no network use.

**Known limitations**

- Windows cannot preselect the chosen account on the sign-in screen.
- Two-user switching not yet verified by hand on Windows, macOS or Linux.
- Windows 10: same native APIs as Windows 11 and type-checked by the same build, but not yet run on a real Windows 10 device. No Acrylic (solid background, no native shadow).
- macOS: switching and locking use the system's private `login.framework` (the component behind the Fast User Switching menu), resolved at runtime; the build is type-checked for both Mac architectures but has not yet run on a real Mac. The app is not notarized.
- Wayland: global shortcut unavailable (use `--toggle`); no background blur on Linux.
- Avatars are initials only.
- Installers are not code-signed.

**Measured (Windows 11 Pro 26300, release build, 2026-11-18)**

- Cold start to visible flyout: 573 ms.
- Idle in tray: main process 26 MB working set; 120–137 MB private memory including the WebView2 processes; 0.4–0.7 % of one core over 30 s.
- Bundle sizes: NSIS 1.26 MiB, MSI 1.80 MiB, DEB/RPM 2.24 MiB, AppImage 77.6 MiB.
