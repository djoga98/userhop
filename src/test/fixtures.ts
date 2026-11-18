import type { AccountEntry } from "../lib/bindings/AccountEntry";
import type { AccountSnapshot } from "../lib/bindings/AccountSnapshot";
import type { PlatformCapabilities } from "../lib/bindings/PlatformCapabilities";
import type { SettingsView } from "../lib/bindings/SettingsView";

export function entry(overrides: Partial<AccountEntry> & Pick<AccountEntry, "key">): AccountEntry {
  return {
    accountId: overrides.key.replace(/^\w+:/, "t:"),
    sessionId: null,
    displayName: "Test User",
    userName: "test",
    isCurrentUser: false,
    presence: "not-signed-in",
    sessions: [],
    availability: { mode: "system-sign-in" },
    recent: false,
    ...overrides,
  };
}

export const currentEntry = entry({
  key: "current:t:1",
  accountId: "t:1",
  sessionId: "t:s:1",
  displayName: "Primary Tester",
  userName: "primary",
  isCurrentUser: true,
  presence: "current",
  sessions: [{ id: "t:s:1", state: "active", kind: "local-graphical", seatId: null }],
  availability: { mode: "current" },
});

export const signedInEntry = entry({
  key: "account:t:2",
  accountId: "t:2",
  displayName: "Second Tester",
  userName: "second",
  presence: "signed-in",
  sessions: [{ id: "t:s:2", state: "inactive", kind: "local-graphical", seatId: null }],
});

export const signedOutEntry = entry({
  key: "account:t:3",
  accountId: "t:3",
  displayName: "Third Tester With A Very Long Display Name That Must Truncate",
  userName: "third",
});

export function snapshot(
  entries: AccountEntry[] = [currentEntry, signedInEntry, signedOutEntry],
): AccountSnapshot {
  return { entries, directoryComplete: true };
}

export function capabilities(overrides: Partial<PlatformCapabilities> = {}): PlatformCapabilities {
  return {
    platform: "windows",
    sessionDiscovery: { status: "available" },
    accountDirectory: { status: "available" },
    systemSignIn: { status: "available" },
    signInSurface: "windows-sign-in",
    sameUserSessionSwitch: { status: "unavailable", reason: "unsupported_platform" },
    lockScreen: { status: "available" },
    backdrop: "none",
    ...overrides,
  };
}

export function settingsView(overrides: Partial<SettingsView> = {}): SettingsView {
  return {
    preferences: { theme: "system", compactRows: false, reduceMotion: false },
    shortcut: { accelerator: "Shift+Super+KeyU", state: "registered" },
    launchAtSignIn: { capability: { status: "available" }, enabled: false },
    defaultShortcut: "Shift+Super+KeyU",
    version: "0.1.0",
    ...overrides,
  };
}
