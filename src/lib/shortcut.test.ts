import { describe, expect, it } from "vitest";

import { captureAccelerator, formatAccelerator } from "./shortcut";

const key = (
  code: string,
  mods: Partial<Record<"ctrl" | "alt" | "shift" | "meta", boolean>> = {},
) => ({
  code,
  ctrlKey: mods.ctrl ?? false,
  altKey: mods.alt ?? false,
  shiftKey: mods.shift ?? false,
  metaKey: mods.meta ?? false,
});

describe("captureAccelerator", () => {
  it("builds accelerators from physical key codes", () => {
    expect(captureAccelerator(key("KeyU", { meta: true, shift: true }))).toEqual({
      kind: "accelerator",
      value: "Shift+Super+KeyU",
    });
    expect(captureAccelerator(key("F7", { ctrl: true, alt: true }))).toEqual({
      kind: "accelerator",
      value: "Control+Alt+F7",
    });
  });

  it("waits on modifiers, cancels on Escape and rejects plain typing", () => {
    expect(captureAccelerator(key("ShiftLeft", { shift: true }))).toEqual({ kind: "pending" });
    expect(captureAccelerator(key("Escape"))).toEqual({ kind: "cancel" });
    expect(captureAccelerator(key("KeyA"))).toEqual({ kind: "invalid" });
    expect(captureAccelerator(key("KeyA", { shift: true }))).toEqual({ kind: "invalid" });
  });
});

describe("formatAccelerator", () => {
  it("uses platform key names in conventional order", () => {
    expect(formatAccelerator("Shift+Super+KeyU", "windows")).toBe("Win + Shift + U");
    expect(formatAccelerator("Shift+Super+KeyU", "linux")).toBe("Super + Shift + U");
    expect(formatAccelerator("Control+Alt+Digit5", "windows")).toBe("Ctrl + Alt + 5");
  });

  it("uses macOS modifier symbols in Apple's order", () => {
    expect(formatAccelerator("Control+Alt+KeyU", "macos")).toBe("⌃⌥U");
    expect(formatAccelerator("Shift+Super+KeyK", "macos")).toBe("⇧⌘K");
    expect(formatAccelerator("Control+Alt+Shift+Super+F5", "macos")).toBe("⌃⌥⇧⌘F5");
  });
});
