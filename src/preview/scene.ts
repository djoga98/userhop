import type { AccountEntry } from "../lib/bindings/AccountEntry";
import type { AccountSnapshot } from "../lib/bindings/AccountSnapshot";
import type { PlatformCapabilities } from "../lib/bindings/PlatformCapabilities";
import type { SettingsView } from "../lib/bindings/SettingsView";
import type { ThemePreference } from "../lib/bindings/ThemePreference";

/** Sample data for README screenshots only; this module is never imported by the app entry. */
export type Scene = {
  theme: Exclude<ThemePreference, "system">;
  view: "list" | "settings";
  focus: number;
  notice: boolean;
  center: boolean;
  hint: boolean;
  width: number;
  height: number;
};

export function readScene(search: string): Scene {
  const params = new URLSearchParams(search);
  const number = (key: string, fallback: number) => Number(params.get(key) ?? fallback) || fallback;
  return {
    theme: params.get("theme") === "light" ? "light" : "dark",
    view: params.get("view") === "settings" ? "settings" : "list",
    focus: number("focus", 0),
    notice: params.get("notice") === "1",
    center: params.get("center") === "1",
    hint: params.get("hint") === "1",
    width: number("w", 1200),
    height: number("h", 640),
  };
}

function account(
  key: string,
  displayName: string,
  userName: string,
  presence: AccountEntry["presence"],
): AccountEntry {
  const current = presence === "current";
  return {
    key: `${current ? "current" : "account"}:${key}`,
    accountId: key,
    sessionId: current ? `${key}:session` : null,
    displayName,
    userName,
    isCurrentUser: current,
    presence,
    sessions: [],
    availability: current ? { mode: "current" } : { mode: "system-sign-in" },
  };
}

export const sampleSnapshot: AccountSnapshot = {
  directoryComplete: true,
  entries: [
    account("sample:1", "Alex Morgan", "alex", "current"),
    account("sample:2", "Sam Rivera", "sam", "signed-in"),
    account("sample:3", "Jordan Lee", "jordan", "not-signed-in"),
    account("sample:4", "Mia Chen", "mia", "not-signed-in"),
  ],
};

export const sampleCapabilities: PlatformCapabilities = {
  platform: "windows",
  sessionDiscovery: { status: "available" },
  accountDirectory: { status: "available" },
  systemSignIn: { status: "available" },
  signInSurface: "windows-sign-in",
  sameUserSessionSwitch: { status: "unavailable", reason: "unsupported_platform" },
  lockScreen: { status: "available" },
  backdrop: "acrylic",
};

export function sampleSettings(scene: Scene): SettingsView {
  return {
    preferences: { theme: scene.theme, compactRows: false, reduceMotion: false },
    shortcut: { accelerator: "Shift+Super+KeyU", state: "registered" },
    launchAtSignIn: { capability: { status: "available" }, enabled: true },
    version: "0.1.0",
  };
}
