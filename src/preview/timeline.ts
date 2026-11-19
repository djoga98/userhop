export const VIDEO_DURATION_MS = 15000;

export const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

export const progress = (t: number, start: number, end: number) =>
  clamp01((t - start) / (end - start));

export const easeOutCubic = (x: number) => 1 - (1 - x) ** 3;

export const easeInOutCubic = (x: number) => (x < 0.5 ? 4 * x * x * x : 1 - (-2 * x + 2) ** 3 / 2);

/** 0 → 1 between inStart..inEnd, holds, then 1 → 0 between outStart..outEnd. */
export function envelope(
  t: number,
  inStart: number,
  inEnd: number,
  outStart: number,
  outEnd: number,
) {
  if (t < inStart || t > outEnd) return 0;
  if (t < inEnd) return easeOutCubic(progress(t, inStart, inEnd));
  if (t > outStart) return 1 - easeInOutCubic(progress(t, outStart, outEnd));
  return 1;
}

export const mix = (from: number, to: number, amount: number) => from + (to - from) * amount;

export type Keystroke = { at: number; keys: string[] };

export const KEYSTROKES: Keystroke[] = [
  { at: 1300, keys: ["Win", "Shift", "U"] },
  { at: 2650, keys: ["↓"] },
  { at: 3450, keys: ["Enter"] },
  { at: 7350, keys: ["Win", "Shift", "U"] },
  { at: 11050, keys: ["Esc"] },
];

export const KEY_STAGGER_MS = 110;
export const KEY_VISIBLE_MS = 1050;

export type FlyoutWindow = { open: number; close: number };

export const FLYOUT_WINDOWS: FlyoutWindow[] = [
  { open: 1900, close: 3750 },
  { open: 7750, close: 11900 },
];

export const SIGN_IN = { in: 4050, out: 6900 };
export const END_CARD = { in: 12300 };
export const CLICKS = { settings: 8750, light: 9550 };

export type Caption = { title: string; body: string; in: number; out: number };

export const CAPTIONS: Caption[] = [
  {
    title: "Switch users\nin one keystroke",
    body: "Press Win + Shift + U from anywhere.",
    in: 250,
    out: 3900,
  },
  {
    title: "Make it yours",
    body: "Light or dark, your own shortcut, launch at sign-in.",
    in: 7500,
    out: 11900,
  },
];
