import type { Platform } from "./bindings/Platform";
import { superKeyName } from "./messages";

type Modifier = "Control" | "Alt" | "Shift" | "Super";

const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
  "OSLeft",
  "OSRight",
]);

const KEY_LABELS: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
};

export type CaptureResult =
  | { kind: "pending" }
  | { kind: "cancel" }
  | { kind: "invalid" }
  | { kind: "accelerator"; value: string };

type KeyLike = Pick<KeyboardEvent, "code" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">;

export function captureAccelerator(event: KeyLike): CaptureResult {
  const hasModifier = event.ctrlKey || event.altKey || event.metaKey || event.shiftKey;
  if (event.code === "Escape" && !hasModifier) return { kind: "cancel" };
  if (MODIFIER_CODES.has(event.code) || event.code === "") return { kind: "pending" };
  if (!(event.ctrlKey || event.altKey || event.metaKey)) return { kind: "invalid" };
  const modifiers: Modifier[] = [];
  if (event.ctrlKey) modifiers.push("Control");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  if (event.metaKey) modifiers.push("Super");
  return { kind: "accelerator", value: [...modifiers, event.code].join("+") };
}

function keyLabel(code: string): string {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^Numpad\d$/.test(code)) return `Num ${code.slice(6)}`;
  return KEY_LABELS[code] ?? code;
}

export function formatAccelerator(accelerator: string, platform: Platform): string {
  const parts = accelerator.split("+");
  const key = parts.pop() ?? "";
  const labels: Record<Modifier, string> = {
    Control: "Ctrl",
    Alt: "Alt",
    Shift: "Shift",
    Super: superKeyName(platform),
  };
  const present = new Set(parts);
  const ordered: Modifier[] = ["Super", "Control", "Alt", "Shift"];
  return [...ordered.filter((m) => present.has(m)).map((m) => labels[m]), keyLabel(key)].join(
    " + ",
  );
}
