import { Lock, ShieldCheck } from "lucide-react";

import type { Platform } from "../lib/bindings/Platform";
import type { ShortcutStatus } from "../lib/bindings/ShortcutStatus";
import { formatAccelerator } from "../lib/shortcut";

type FooterProps = {
  shortcut: ShortcutStatus | null;
  platform: Platform;
  lockAvailable: boolean;
  busy: boolean;
  onLock: () => void;
};

function shortcutHint(shortcut: ShortcutStatus | null, platform: Platform) {
  if (!shortcut) return { text: "", warn: false };
  switch (shortcut.state) {
    case "registered":
      return {
        text: shortcut.accelerator ? formatAccelerator(shortcut.accelerator, platform) : "",
        warn: false,
      };
    case "disabled":
      return { text: "Open from the tray", warn: false };
    case "conflict":
    case "invalid":
    case "unsupported":
      return { text: "Shortcut unavailable — open from tray", warn: true };
  }
}

export function Footer({ shortcut, platform, lockAvailable, busy, onLock }: FooterProps) {
  const hint = shortcutHint(shortcut, platform);
  return (
    <footer className="flex h-11 shrink-0 items-center justify-between gap-2 border-t border-separator px-4 text-[12px] text-secondary">
      <span className={`truncate ${hint.warn ? "text-warning" : ""}`}>{hint.text}</span>
      <span className="flex shrink-0 items-center gap-1">
        <ShieldCheck aria-hidden="true" size={14} />
        <span>Secure OS sign-in</span>
        {lockAvailable && (
          <button
            type="button"
            aria-label="Lock screen"
            title="Lock screen"
            aria-disabled={busy || undefined}
            onClick={() => {
              if (!busy) onLock();
            }}
            className="ml-2 grid size-7 place-items-center rounded-md hover:bg-hover hover:text-text active:bg-pressed"
          >
            <Lock aria-hidden="true" size={14} />
          </button>
        )}
      </span>
    </footer>
  );
}
