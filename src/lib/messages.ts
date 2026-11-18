import type { AccountEntry } from "./bindings/AccountEntry";
import type { ErrorCode } from "./bindings/ErrorCode";
import type { Platform } from "./bindings/Platform";
import type { Presence } from "./bindings/Presence";
import type { SignInSurface } from "./bindings/SignInSurface";

export const errorMessages: Record<ErrorCode, string> = {
  permission_denied: "The system did not allow this action.",
  session_not_found: "That account is no longer available. The list was refreshed.",
  session_no_longer_active: "That session has ended. The list was refreshed.",
  not_local_graphical: "Switching only works from a local desktop session.",
  remote_session: "Switching isn't available in a remote session.",
  policy_disabled: "Your administrator has turned this off.",
  unsupported_platform: "This operating system isn't supported.",
  unsupported_desktop: "This desktop doesn't support switching from here.",
  unsupported_login_manager:
    "The login screen can't be opened from here. Use your desktop's Switch User menu.",
  secure_handoff_unavailable:
    "Your session couldn't be locked first, so nothing was switched. Lock the screen and switch from there.",
  shortcut_conflict: "Shortcut unavailable — open from tray or change the shortcut.",
  shortcut_invalid: "Use a combination that includes Ctrl, Alt or the Windows/Super key.",
  shortcut_unsupported: "Global shortcuts aren't available on this desktop.",
  autostart_failed: "The startup setting couldn't be changed.",
  settings_io: "Settings couldn't be saved.",
  os_api_failure: "The system reported an error.",
  invalid_request: "That request wasn't valid.",
  busy: "Another request is still in progress.",
  unknown: "Something went wrong.",
};

export const staleCodes: readonly ErrorCode[] = ["session_not_found", "session_no_longer_active"];

const presenceLabels: Record<Presence, string> = {
  current: "Current session",
  "own-session": "Your other session",
  "signed-in": "Signed in",
  "signed-in-remote": "Signed in remotely",
  "not-signed-in": "Not signed in",
};

export function statusLabel(entry: AccountEntry): string {
  if (entry.availability.mode === "unavailable" && !entry.isCurrentUser) {
    return `${presenceLabels[entry.presence]} · System sign-in required`;
  }
  return presenceLabels[entry.presence];
}

export function actionLabel(entry: AccountEntry): string | null {
  switch (entry.availability.mode) {
    case "current":
      return null;
    case "direct":
      return "Switch";
    case "system-sign-in":
      return entry.presence === "signed-in" ? "Switch" : "Open sign-in";
    case "unavailable":
      return null;
  }
}

export function handoffDescription(surface: SignInSurface): string {
  switch (surface) {
    case "windows-sign-in":
      return "Opens the Windows sign-in screen. Your apps keep running.";
    case "gdm":
    case "freedesktop-display-manager":
      return "Locks this session and opens the login screen. Your apps keep running.";
    case "none":
      return "System sign-in isn't available here.";
  }
}

export function superKeyName(platform: Platform): string {
  return platform === "windows" ? "Win" : "Super";
}
