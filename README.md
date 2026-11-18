# Fluent Account Switcher

A small tray utility for Windows 11 and Linux that opens a Windows 11 style flyout (similar to the `Win + V` clipboard panel), lists the real accounts and sessions on the machine, and hands off to the operating system's own sign-in screen to switch.

It never asks for, receives or stores passwords, PINs or tokens. Authentication is always done by Windows or by your Linux login manager.

## What it does

- Opens with **Win + Shift + U** (configurable) or from the tray icon. The tray menu offers *Open account switcher*, *Lock screen*, *Settings* and *Quit*.
- Shows the current account first, then other accounts with an honest status: *Current session*, *Signed in*, *Signed in remotely*, *Not signed in*.
- Selecting another account hides the flyout and asks the OS to switch:
  - **Windows:** the current session is disconnected (it keeps running) and Windows shows its sign-in screen. Windows does not preselect the account you picked; choose it there and sign in.
  - **Linux:** the app locks your session, waits for logind to confirm the lock, then opens the login screen of GDM, LightDM or SDDM. Another user's session is never activated directly. Your own second session on the same seat can be activated directly.
- If switching is not possible (remote session, administrator policy, unsupported login manager, Wayland shortcut limits), the flyout says why and offers what does work.
- Settings: launch at sign-in, shortcut, theme (System/Light/Dark), compact rows, reduce motion. Preferences are stored per user in the app config folder; no account lists, session IDs or names are stored.
- No network access, telemetry or remote content.

## Platform status

| Platform | Status |
| --- | --- |
| Windows 11 (x64) | Primary target. Builds, runs, lists accounts, shortcut and tray verified on Windows 11 Pro 26300. Switching to another account has not yet been verified with two real accounts. |
| Linux with systemd-logind | Builds and runs. Session discovery verified against logind. Greeter handoff on GNOME/GDM, KDE/SDDM and LightDM is implemented but not yet verified on a real desktop. |
| Linux on Wayland | Global shortcut is reported unavailable. Bind `fluent-account-switcher --toggle` as a custom shortcut in your desktop's keyboard settings instead. No background blur on Linux. |
| Windows 10, macOS, non-systemd Linux | Not supported. |

## Install

Release bundles are produced by CI or locally with `pnpm tauri build`. They are **not code-signed**, so Windows SmartScreen may warn before running them.

- **Windows:** run `Fluent Account Switcher_<version>_x64-setup.exe` (per-user install, no administrator rights) or the `.msi`. WebView2 is already part of Windows 11.
- **Debian/Ubuntu:** `sudo apt install "./Fluent Account Switcher_<version>_amd64.deb"`
- **Fedora/openSUSE:** `sudo dnf install "./Fluent Account Switcher-<version>-1.x86_64.rpm"`
- **AppImage:** `chmod +x Fluent*.AppImage && ./Fluent*.AppImage`

The app starts in the tray. Launch at sign-in is off until you turn it on in Settings.

## Uninstall

- **Windows:** Settings → Apps → Installed apps → *Fluent Account Switcher* → Uninstall. Tick *Delete the application data* to also remove `%APPDATA%\com.fluentaccountswitcher.desktop` (settings) and `%LOCALAPPDATA%\com.fluentaccountswitcher.desktop` (WebView2 cache). Turn off *Launch at sign-in* first, or remove the `Fluent Account Switcher` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- **Linux:** remove the package (`sudo apt remove fluent-account-switcher` / `sudo dnf remove fluent-account-switcher`), then optionally delete `~/.config/com.fluentaccountswitcher.desktop`, `~/.local/share/com.fluentaccountswitcher.desktop` and `~/.config/autostart/Fluent Account Switcher.desktop`.

Nothing else is installed: no services, scheduled tasks, setuid helpers, polkit rules or login extensions.

## Build from source

Prerequisites follow the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/):

- Node.js LTS (24 or newer) and pnpm 12 (`corepack enable` or `npx pnpm@12.21.0`)
- Rust stable (1.90 or newer)
- Windows: Visual Studio Build Tools with the C++ workload, WebView2 (included in Windows 11)
- Debian/Ubuntu: `sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev build-essential file patchelf`

```sh
pnpm install --frozen-lockfile
pnpm tauri dev          # run the desktop app with hot reload
pnpm tauri build        # release binary and installers in src-tauri/target/release/bundle
```

### Checks

```sh
pnpm lint
pnpm typecheck
pnpm test --run
pnpm build
pnpm format:check

cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test -- --ignored native_   # read-only check against the real OS session APIs
```

`cargo test` also regenerates the TypeScript contract in `src/lib/bindings/` from the Rust models; CI fails if it is out of date.

## Command-line arguments

| Argument | Effect |
| --- | --- |
| `--toggle` | Toggle the flyout of the already running instance (useful for desktop shortcuts on Wayland). |
| `--hidden` | Start in the tray without showing the flyout. |
| `--autostart` | Used by the launch-at-sign-in entry; starts hidden. |

## Project layout

```text
src/                 React + TypeScript flyout (strict, Tailwind CSS tokens)
  app/               shell: header, footer, view switching, theming, window fit
  features/accounts/ state machine, list, rows
  features/settings/ settings view and shortcut capture
  lib/               typed Tauri commands, generated bindings, helpers
src-tauri/src/
  session/           provider trait, policy, ID validation, service
  platform/          Windows (WTS/NetAPI) and Linux (logind/AccountsService/DM) adapters
  shell/             flyout window, positioning, tray, shortcut, backdrop
  settings.rs        versioned per-user preferences
```

## License

MIT. See [LICENSE](LICENSE).
