import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AccountSnapshot } from "./bindings/AccountSnapshot";
import type { ErrorCode } from "./bindings/ErrorCode";
import type { FlyoutOpened } from "./bindings/FlyoutOpened";
import type { PlatformCapabilities } from "./bindings/PlatformCapabilities";
import type { Preferences } from "./bindings/Preferences";
import type { SettingsView } from "./bindings/SettingsView";
import type { SwitchRequest } from "./bindings/SwitchRequest";
import type { SwitchResult } from "./bindings/SwitchResult";

const ERROR_CODES: readonly ErrorCode[] = [
  "permission_denied",
  "session_not_found",
  "session_no_longer_active",
  "not_local_graphical",
  "remote_session",
  "policy_disabled",
  "unsupported_platform",
  "unsupported_desktop",
  "unsupported_login_manager",
  "secure_handoff_unavailable",
  "shortcut_conflict",
  "shortcut_invalid",
  "shortcut_unsupported",
  "autostart_failed",
  "settings_io",
  "os_api_failure",
  "invalid_request",
  "busy",
  "unknown",
];

export class CommandError extends Error {
  readonly code: ErrorCode;

  constructor(code: ErrorCode) {
    super(code);
    this.name = "CommandError";
    this.code = code;
  }
}

function isErrorCode(value: unknown): value is ErrorCode {
  return typeof value === "string" && (ERROR_CODES as readonly string[]).includes(value);
}

export function toErrorCode(error: unknown): ErrorCode {
  if (error instanceof CommandError) return error.code;
  if (typeof error === "object" && error !== null && "code" in error) {
    const code: unknown = error.code;
    if (isErrorCode(code)) return code;
  }
  return "unknown";
}

async function call<T>(command: string, args?: InvokeArgs): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw new CommandError(toErrorCode(error));
  }
}

export const api = {
  getAccounts: () => call<AccountSnapshot>("get_accounts"),
  getCapabilities: () => call<PlatformCapabilities>("get_capabilities"),
  requestSwitch: (request: SwitchRequest) => call<SwitchResult>("request_switch", { request }),
  openSystemSignIn: () => call<SwitchResult>("open_system_sign_in"),
  lockSession: () => call<null>("lock_session"),
  getSettings: () => call<SettingsView>("get_settings"),
  saveSettings: (preferences: Preferences) => call<SettingsView>("save_settings", { preferences }),
  setShortcut: (accelerator: string | null) => call<SettingsView>("set_shortcut", { accelerator }),
  setLaunchAtSignIn: (enabled: boolean) => call<SettingsView>("set_launch_at_sign_in", { enabled }),
  setShortcutCapture: (active: boolean) => call<null>("set_shortcut_capture", { active }),
  hideFlyout: () => call<null>("hide_flyout"),
  fitFlyout: (height: number) => call<null>("fit_flyout", { height }),
};

export const events = {
  onFlyoutOpened: (handler: (payload: FlyoutOpened) => void): Promise<UnlistenFn> =>
    listen<FlyoutOpened>("flyout://opened", (event) => {
      handler(event.payload);
    }),
  onFlyoutHidden: (handler: () => void): Promise<UnlistenFn> =>
    listen("flyout://hidden", () => {
      handler();
    }),
  onSessionsChanged: (handler: () => void): Promise<UnlistenFn> =>
    listen("sessions://changed", () => {
      handler();
    }),
};
