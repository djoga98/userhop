import { emit } from "@tauri-apps/api/event";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { installBackend, rejectWith } from "../test/backend";
import {
  capabilities,
  currentEntry,
  entry,
  settingsView,
  signedInEntry,
  snapshot,
} from "../test/fixtures";
import { setMedia } from "../test/setup";
import { App } from "./App";

async function renderApp() {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("list", { name: "Accounts" });
  return user;
}

function row(name: RegExp | string) {
  return screen.getByRole("button", { name });
}

describe("account list", () => {
  it("shows the real backend entries with the current user first (UI-04)", async () => {
    installBackend();
    await renderApp();
    const items = within(screen.getByRole("list", { name: "Accounts" })).getAllByRole("button");
    expect(items).toHaveLength(3);
    expect(items[0]).toHaveAccessibleName("Primary Tester (primary)");
    expect(items[0]).toHaveAccessibleDescription(/Current session/);
    expect(row(/Second Tester/)).toHaveAccessibleDescription(/Signed in.*Switch.*Windows sign-in/);
    expect(row(/Third Tester/)).toHaveAccessibleDescription(/Not signed in.*Open sign-in/);
    expect(row(/Third Tester/)).toHaveAttribute("title", expect.stringContaining("Very Long"));
  });

  it("renders an error with safe fallbacks instead of sample rows (UI-06)", async () => {
    const backend = installBackend({ get_accounts: rejectWith("permission_denied") });
    const user = userEvent.setup();
    render(<App />);
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("The system did not allow this action.");
    expect(screen.queryByRole("list", { name: "Accounts" })).not.toBeInTheDocument();
    await user.click(within(alert).getByRole("button", { name: "Open system sign-in" }));
    expect(backend.count("open_system_sign_in")).toBe(1);
  });

  it("retries enumeration from the error state", async () => {
    let fail = true;
    installBackend({
      get_accounts: () => {
        if (fail) throw { code: "os_api_failure" };
        return snapshot();
      },
    });
    const user = userEvent.setup();
    render(<App />);
    const alert = await screen.findByRole("alert");
    fail = false;
    await user.click(within(alert).getByRole("button", { name: "Try again" }));
    expect(await screen.findByRole("list", { name: "Accounts" })).toBeInTheDocument();
  });

  it("offers native sign-in when no other accounts exist (UI-05)", async () => {
    const backend = installBackend({ get_accounts: () => snapshot([currentEntry]) });
    const user = await renderApp();
    await user.click(screen.getByRole("button", { name: /Use another account/ }));
    expect(backend.count("open_system_sign_in")).toBe(1);
  });

  it("never renders credential inputs (SEC-02)", async () => {
    installBackend();
    const user = await renderApp();
    expect(document.querySelector("input[type=password]")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await screen.findByRole("switch", { name: "Launch at sign-in" });
    expect(document.querySelector("input[type=password]")).toBeNull();
  });
});

describe("switching", () => {
  it("sends exactly one request for rapid repeated activation (UI-07)", async () => {
    let release: () => void = () => undefined;
    const backend = installBackend({
      request_switch: () =>
        new Promise((resolve) => {
          release = () => {
            resolve({ kind: "handoff-requested", mode: "system-sign-in" });
          };
        }),
    });
    const user = await renderApp();
    const target = row(/Second Tester/);
    await user.click(target);
    await user.click(target);
    await user.keyboard("{Enter}");
    await user.click(row(/Third Tester/));
    expect(backend.count("request_switch")).toBe(1);
    expect(backend.last("request_switch")?.args).toEqual({
      request: { accountId: "t:2", sessionId: null },
    });
    await act(async () => {
      release();
      await Promise.resolve();
    });
  });

  it("selecting the current session never contacts the OS (OS-05)", async () => {
    const backend = installBackend();
    const user = await renderApp();
    await user.click(row(/Primary Tester/));
    expect(backend.count("request_switch")).toBe(0);
  });

  it("explains unavailable targets without calling the backend", async () => {
    const blocked = entry({
      key: "account:t:9",
      displayName: "Remote Tester",
      presence: "signed-in-remote",
      availability: { mode: "unavailable", reason: "remote_session" },
    });
    const backend = installBackend({ get_accounts: () => snapshot([currentEntry, blocked]) });
    const user = await renderApp();
    expect(row(/Remote Tester/)).toHaveAccessibleDescription(/System sign-in required/);
    await user.click(row(/Remote Tester/));
    expect(await screen.findByRole("alert")).toHaveTextContent("remote session");
    expect(backend.count("request_switch")).toBe(0);
  });

  it("refreshes after a stale target is rejected (OS-04)", async () => {
    const backend = installBackend({ request_switch: rejectWith("session_no_longer_active") });
    const user = await renderApp();
    const before = backend.count("get_accounts");
    await user.click(row(/Second Tester/));
    expect(await screen.findByRole("alert")).toHaveTextContent("session has ended");
    await waitFor(() => {
      expect(backend.count("get_accounts")).toBe(before + 1);
    });
  });

  it("shows the OS refusal and allows a manual retry, never an automatic one (OS-07)", async () => {
    const backend = installBackend({ request_switch: rejectWith("permission_denied") });
    const user = await renderApp();
    await user.click(row(/Second Tester/));
    const alert = await screen.findByRole("alert");
    expect(backend.count("request_switch")).toBe(1);
    await user.click(within(alert).getByRole("button", { name: "Try again" }));
    await waitFor(() => {
      expect(backend.count("request_switch")).toBe(2);
    });
  });

  it("offers locking when the session could not be secured first", async () => {
    const backend = installBackend({ request_switch: rejectWith("secure_handoff_unavailable") });
    const user = await renderApp();
    await user.click(row(/Second Tester/));
    const alert = await screen.findByRole("alert");
    expect(within(alert).queryByRole("button", { name: "Try again" })).toBeNull();
    await user.click(within(alert).getByRole("button", { name: "Lock screen" }));
    expect(backend.count("lock_session")).toBe(1);
  });
});

describe("keyboard and window behavior (UI-03)", () => {
  it("focuses the first row on open, moves with arrows and activates with Enter", async () => {
    const backend = installBackend();
    const user = await renderApp();
    await waitFor(() => {
      expect(row(/Primary Tester/)).toHaveFocus();
    });
    await user.keyboard("{ArrowDown}");
    expect(row(/Second Tester/)).toHaveFocus();
    await user.keyboard("{End}");
    expect(row(/Third Tester/)).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(row(/Third Tester/)).toHaveFocus();
    await user.keyboard("{Home}{ArrowDown}{Enter}");
    expect(backend.last("request_switch")?.args).toEqual({
      request: { accountId: "t:2", sessionId: null },
    });
  });

  it("Escape returns from settings, then closes the flyout", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const backend = installBackend();
    const user = await renderApp();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.getByRole("heading", { name: "Switch account" })).toBeInTheDocument();
    await user.keyboard("{Escape}");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(backend.count("hide_flyout")).toBe(1);
    vi.useRealTimers();
  });

  it("reopening resets to the requested view and refreshes accounts", async () => {
    const backend = installBackend();
    await renderApp();
    const before = backend.count("get_accounts");
    await act(async () => {
      await emit("flyout://opened", { view: "settings" });
    });
    expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
    expect(backend.count("get_accounts")).toBe(before + 1);
  });

  it("refreshes on OS session changes while visible", async () => {
    const backend = installBackend();
    await renderApp();
    const before = backend.count("get_accounts");
    await act(async () => {
      await emit("sessions://changed");
      await emit("sessions://changed");
    });
    await waitFor(() => {
      expect(backend.count("get_accounts")).toBe(before + 1);
    });
  });

  it("does not refresh while hidden (PERF-01)", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const backend = installBackend();
    await renderApp();
    await act(async () => {
      await emit("flyout://hidden");
    });
    const before = backend.count("get_accounts");
    await act(async () => {
      await emit("sessions://changed");
      await vi.advanceTimersByTimeAsync(500);
    });
    expect(backend.count("get_accounts")).toBe(before);
    vi.useRealTimers();
  });
});

describe("appearance (UI-08, UI-09)", () => {
  it("follows the system theme and honors an explicit choice", async () => {
    setMedia("(prefers-color-scheme: dark)", true);
    installBackend({
      get_settings: () =>
        settingsView({ preferences: { theme: "light", compactRows: false, reduceMotion: false } }),
    });
    const { container } = render(<App />);
    await screen.findByRole("list", { name: "Accounts" });
    await waitFor(() => {
      expect(container.firstElementChild).toHaveAttribute("data-resolved-theme", "light");
    });
  });

  it("uses dark tokens when the system prefers dark", async () => {
    setMedia("(prefers-color-scheme: dark)", true);
    installBackend();
    const { container } = render(<App />);
    await screen.findByRole("list", { name: "Accounts" });
    await waitFor(() => {
      expect(container.firstElementChild).toHaveAttribute("data-resolved-theme", "dark");
    });
  });

  it("reduces motion from the OS preference or the setting", async () => {
    setMedia("(prefers-reduced-motion: reduce)", true);
    installBackend();
    const { container } = render(<App />);
    await screen.findByRole("list", { name: "Accounts" });
    expect(container.firstElementChild).toHaveAttribute("data-reduced-motion", "true");
  });

  it("marks the native acrylic backdrop only when the backend reports it", async () => {
    installBackend({ get_capabilities: () => capabilities({ backdrop: "acrylic" }) });
    const { container } = render(<App />);
    await screen.findByRole("list", { name: "Accounts" });
    await waitFor(() => {
      expect(container.firstElementChild).toHaveAttribute("data-backdrop", "acrylic");
    });
  });
});

describe("settings (UI-12)", () => {
  it("persists preference changes through the backend", async () => {
    const backend = installBackend();
    const user = await renderApp();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(await screen.findByRole("switch", { name: "Compact rows" }));
    await waitFor(() => {
      expect(backend.last("save_settings")?.args).toEqual({
        preferences: { theme: "system", compactRows: true, reduceMotion: false },
      });
    });
    await user.click(screen.getByRole("radio", { name: "Dark" }));
    await waitFor(() => {
      expect(backend.last("save_settings")?.args).toEqual({
        preferences: { theme: "dark", compactRows: true, reduceMotion: false },
      });
    });
    expect(screen.getByRole("switch", { name: "Compact rows" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });

  it("toggles launch at sign-in", async () => {
    const backend = installBackend();
    const user = await renderApp();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(await screen.findByRole("switch", { name: "Launch at sign-in" }));
    await waitFor(() => {
      expect(backend.last("set_launch_at_sign_in")?.args).toEqual({ enabled: true });
    });
  });

  it("captures a new shortcut while suspending the global one", async () => {
    const backend = installBackend();
    const user = await renderApp();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(await screen.findByRole("button", { name: /Shortcut/ }));
    await waitFor(() => {
      expect(backend.last("set_shortcut_capture")?.args).toEqual({ active: true });
    });
    await user.keyboard("{Control>}{Alt>}k{/Alt}{/Control}");
    await waitFor(() => {
      expect(backend.last("set_shortcut")?.args).toEqual({ accelerator: "Control+Alt+KeyK" });
    });
    expect(backend.last("set_shortcut_capture")?.args).toEqual({ active: false });
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });

  it("Escape cancels shortcut capture without leaving settings", async () => {
    const backend = installBackend();
    const user = await renderApp();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(await screen.findByRole("button", { name: /Shortcut/ }));
    await user.keyboard("{Escape}");
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
    expect(backend.count("set_shortcut")).toBe(0);
  });

  it("reports a shortcut conflict and keeps the tray fallback visible", async () => {
    installBackend({
      get_settings: () =>
        settingsView({ shortcut: { accelerator: "Shift+Super+KeyU", state: "conflict" } }),
      set_shortcut: rejectWith("shortcut_conflict"),
    });
    const user = await renderApp();
    expect(screen.getByText("Shortcut unavailable — open from tray")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(await screen.findByRole("button", { name: "Reset to default" }));
    expect(
      await screen.findAllByText(/Shortcut unavailable — open from tray or change/),
    ).not.toHaveLength(0);
  });

  it("explains Wayland shortcut limits with the supported command fallback", async () => {
    installBackend({
      get_capabilities: () => capabilities({ platform: "linux", backdrop: "none" }),
      get_settings: () =>
        settingsView({ shortcut: { accelerator: "Shift+Super+KeyU", state: "unsupported" } }),
    });
    const user = await renderApp();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    expect(await screen.findByText(/fluent-account-switcher --toggle/)).toBeInTheDocument();
  });
});

describe("own sessions", () => {
  it("offers direct activation only for the user's own session", async () => {
    const own = entry({
      key: "session:t:s:7",
      accountId: "t:1",
      sessionId: "t:s:7",
      displayName: "Primary Tester",
      userName: "primary",
      isCurrentUser: true,
      presence: "own-session",
      availability: { mode: "direct", sessionId: "t:s:7" },
    });
    const backend = installBackend({
      get_accounts: () => snapshot([currentEntry, own, signedInEntry]),
      request_switch: () => ({ kind: "handoff-requested", mode: "direct" }),
    });
    const user = await renderApp();
    const rows = within(screen.getByRole("list", { name: "Accounts" })).getAllByRole("button");
    expect(rows[1]).toHaveAccessibleDescription(/Your other session/);
    await user.click(rows[1] as HTMLElement);
    expect(backend.last("request_switch")?.args).toEqual({
      request: { accountId: "t:1", sessionId: "t:s:7" },
    });
  });
});

describe("focus continuity", () => {
  it("moves focus into settings and back to the list", async () => {
    installBackend();
    const user = await renderApp();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Back to accounts" })).toHaveFocus();
    });
    await user.keyboard("{Escape}");
    await waitFor(() => {
      expect(row(/Primary Tester/)).toHaveFocus();
    });
  });
});

describe("macOS", () => {
  it("shows the login window hint, Mac shortcut symbols and the platform default", async () => {
    const backend = installBackend({
      get_capabilities: () =>
        capabilities({
          platform: "macos",
          signInSurface: "macos-login-window",
          backdrop: "vibrancy",
        }),
      get_settings: () =>
        settingsView({
          shortcut: { accelerator: "Shift+Super+KeyK", state: "registered" },
          defaultShortcut: "Control+Alt+KeyU",
        }),
    });
    const user = await renderApp();
    expect(row(/Second Tester/)).toHaveAccessibleDescription(/macOS login window/);
    expect(screen.getByText("⇧⌘K")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(await screen.findByRole("button", { name: "Reset to default" }));
    await waitFor(() => {
      expect(backend.last("set_shortcut")?.args).toEqual({ accelerator: "Control+Alt+KeyU" });
    });
  });
});

describe("Windows 10", () => {
  it("uses the solid fallback when the backend reports no native backdrop", async () => {
    installBackend({
      get_capabilities: () => capabilities({ platform: "windows", backdrop: "none" }),
    });
    const { container } = render(<App />);
    await screen.findByRole("list", { name: "Accounts" });
    await waitFor(() => {
      expect(container.firstElementChild).toHaveAttribute("data-platform", "windows");
    });
    expect(container.firstElementChild).toHaveAttribute("data-backdrop", "none");
  });
});
