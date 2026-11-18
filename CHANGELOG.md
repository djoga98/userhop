# Changelog

## 0.1.0 — unreleased (2026-11-18)

**Verified targets:** Windows 11 Pro build 26300 x64 (read-only session enumeration test, desktop smoke test of the debug and release builds). Linux build and logind enumeration verified on Ubuntu 24.04 under WSL2 only; no real GNOME/KDE desktop tested yet.

**Features**

- Tray-resident flyout styled after the Windows 11 clipboard panel, with native Acrylic on Windows 11 and a solid Fluent-tinted fallback elsewhere; light, dark and system themes; high-contrast (`forced-colors`) styling; reduced motion.
- Real account and session discovery: Windows WTS sessions plus local accounts from NetAPI (disabled and sign-in-hidden accounts excluded); Linux logind sessions plus AccountsService users.
- Switching hands off to the OS: Windows sign-in screen via `WTSDisconnectSession` of the own console session; Linux login screen (GDM, LightDM, SDDM) after a confirmed lock of the own session; direct activation only for the user's own second session.
- Lock screen action (tray, footer).
- Global shortcut (default Win/Super + Shift + U) with conflict detection and in-app capture; `--toggle` for desktops without global shortcuts.
- Launch at sign-in (per user, off by default).
- Keyboard: arrows/Home/End move between rows, Enter/Space activate, Escape goes back then closes, focus moves into Settings and back.

**Security behavior**

OS authentication remains native. The app never accepts or stores credentials, exposes twelve narrow commands guarded by Tauri app permissions, revalidates every switch request against a fresh OS snapshot, allows one switch request at a time and never retries automatically. Strict CSP, no remote content, no network use.

**Known limitations**

- Windows cannot preselect the chosen account on the sign-in screen.
- Two-user switching not yet verified by hand on Windows or Linux.
- Wayland: global shortcut unavailable (use `--toggle`); no background blur on Linux.
- Avatars are initials only.
- Installers are not code-signed.

**Measured (Windows 11 Pro 26300, release build, 2026-11-18)**

- Cold start to visible flyout: 573 ms.
- Idle in tray: main process 26 MB working set; 120–137 MB private memory including the WebView2 processes; 0.4–0.7 % of one core over 30 s.
- Bundle sizes: NSIS 1.26 MiB, MSI 1.80 MiB, DEB/RPM 2.24 MiB, AppImage 77.6 MiB.
