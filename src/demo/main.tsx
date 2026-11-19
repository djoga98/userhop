import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "../app/App";
import type { AccountEntry } from "../lib/bindings/AccountEntry";
import type { AccountSnapshot } from "../lib/bindings/AccountSnapshot";
import type { Preferences } from "../lib/bindings/Preferences";
import type { SettingsView } from "../lib/bindings/SettingsView";
import type { SwitchRequest } from "../lib/bindings/SwitchRequest";
import type { SwitchResult } from "../lib/bindings/SwitchResult";
import { readScene, sampleCapabilitiesFor, sampleSnapshot } from "../preview/scene";
import "../styles/index.css";

/** Messages exchanged with the landing page that hosts this demo in an iframe. */
export type DemoToPage =
  | { type: "height"; value: number }
  | { type: "hidden" }
  | { type: "handoff"; name: string }
  | { type: "lock" };

export type PageToDemo = { type: "open" } | { type: "close" };

const capabilities = sampleCapabilitiesFor(readScene(""));
let recentAccountId: string | null = null;
let settings: SettingsView = {
  preferences: { theme: "system", compactRows: false, reduceMotion: false },
  shortcut: { accelerator: "Shift+Super+KeyU", state: "registered" },
  launchAtSignIn: { capability: { status: "available" }, enabled: false },
  defaultShortcut: "Shift+Super+KeyU",
  version: "0.1.0",
};

function post(message: DemoToPage) {
  window.parent.postMessage(message, window.location.origin);
}

function snapshot(): AccountSnapshot {
  const entries = sampleSnapshot.entries.map((entry) => ({
    ...entry,
    recent: entry.accountId === recentAccountId,
  }));
  const rank = (entry: AccountEntry) => (entry.isCurrentUser ? 0 : entry.recent ? 1 : 2);
  return { ...sampleSnapshot, entries: entries.sort((a, b) => rank(a) - rank(b)) };
}

function handOff(message: DemoToPage, result: SwitchResult): SwitchResult {
  post(message);
  setTimeout(() => {
    void emit("flyout://hidden");
  }, 0);
  return result;
}

function requestSwitch(request: SwitchRequest): SwitchResult {
  const target = sampleSnapshot.entries.find((entry) => entry.accountId === request.accountId);
  if (!target || target.isCurrentUser) return { kind: "no-op-current-session" };
  recentAccountId = target.accountId;
  return handOff(
    { type: "handoff", name: target.displayName },
    { kind: "handoff-requested", mode: "system-sign-in" },
  );
}

mockIPC(
  (command, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    switch (command) {
      case "get_accounts":
        return snapshot();
      case "get_capabilities":
        return capabilities;
      case "get_settings":
        return settings;
      case "save_settings":
        settings = { ...settings, preferences: args.preferences as Preferences };
        return settings;
      case "set_shortcut":
        settings = {
          ...settings,
          shortcut:
            typeof args.accelerator === "string"
              ? { accelerator: args.accelerator, state: "registered" }
              : { accelerator: null, state: "disabled" },
        };
        return settings;
      case "set_launch_at_sign_in":
        settings = {
          ...settings,
          launchAtSignIn: { ...settings.launchAtSignIn, enabled: args.enabled === true },
        };
        return settings;
      case "request_switch":
        return requestSwitch(args.request as SwitchRequest);
      case "open_system_sign_in":
        return handOff(
          { type: "handoff", name: "Other user" },
          { kind: "handoff-requested", mode: "system-sign-in" },
        );
      case "lock_session":
        handOff({ type: "lock" }, { kind: "no-op-current-session" });
        return null;
      case "hide_flyout":
        post({ type: "hidden" });
        void emit("flyout://hidden");
        return null;
      case "fit_flyout":
      case "flyout_ready":
        post({ type: "height", value: Number(args.height) });
        return null;
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);

// The app focuses a row when it mounts; inside an iframe that would scroll the hosting page to
// the demo on load. Programmatic focus stays off until the visitor engages with the demo.
const focusDescriptor = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "focus");
const nativeFocus: unknown = focusDescriptor?.value;
let engaged = false;
HTMLElement.prototype.focus = function focus(this: HTMLElement, options?: FocusOptions) {
  if (engaged && typeof nativeFocus === "function") {
    Reflect.apply(nativeFocus, this, [{ ...options, preventScroll: true }]);
  }
};
const engage = () => {
  engaged = true;
};
document.addEventListener("pointerdown", engage, { capture: true });
document.addEventListener("keydown", engage, { capture: true });

window.addEventListener("message", (event: MessageEvent<PageToDemo>) => {
  if (event.origin !== window.location.origin || event.source !== window.parent) return;
  if (event.data.type === "open") {
    engage();
    void emit("flyout://opened", { view: "list" });
  }
  if (event.data.type === "close") void emit("flyout://hidden");
});

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
