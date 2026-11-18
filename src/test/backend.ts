import { mockIPC } from "@tauri-apps/api/mocks";
import type { InvokeArgs } from "@tauri-apps/api/core";

import { capabilities, settingsView, snapshot } from "./fixtures";

type Handler = (args: Record<string, unknown>) => unknown;

export type Call = { command: string; args: Record<string, unknown> };

/** Test-only fake of the Rust command surface; never bundled into production code. */
export function installBackend(overrides: Record<string, Handler> = {}) {
  const calls: Call[] = [];
  const handlers: Record<string, Handler> = {
    get_accounts: () => snapshot(),
    get_capabilities: () => capabilities(),
    get_settings: () => settingsView(),
    save_settings: (args) =>
      settingsView({
        preferences: args.preferences as ReturnType<typeof settingsView>["preferences"],
      }),
    set_shortcut: (args) =>
      settingsView({
        shortcut: {
          accelerator: (args.accelerator as string | null) ?? null,
          state: args.accelerator ? "registered" : "disabled",
        },
      }),
    set_launch_at_sign_in: (args) =>
      settingsView({
        launchAtSignIn: { capability: { status: "available" }, enabled: Boolean(args.enabled) },
      }),
    request_switch: () => ({ kind: "handoff-requested", mode: "system-sign-in" }),
    open_system_sign_in: () => ({ kind: "handoff-requested", mode: "system-sign-in" }),
    lock_session: () => null,
    set_shortcut_capture: () => null,
    hide_flyout: () => null,
    fit_flyout: () => null,
    flyout_ready: () => null,
    ...overrides,
  };
  mockIPC(
    (command: string, payload?: InvokeArgs) => {
      const args = (payload ?? {}) as Record<string, unknown>;
      calls.push({ command, args });
      const handler = handlers[command];
      if (!handler) throw new Error(`unexpected command ${command}`);
      return handler(args);
    },
    { shouldMockEvents: true },
  );
  return {
    calls,
    count: (command: string) => calls.filter((c) => c.command === command).length,
    last: (command: string) => calls.filter((c) => c.command === command).at(-1),
  };
}

export function rejectWith(code: string): Handler {
  return () => {
    throw { code };
  };
}
