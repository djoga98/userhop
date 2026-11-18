import { useEffect, useState, type KeyboardEvent } from "react";

import { api } from "../../lib/api";
import type { Platform } from "../../lib/bindings/Platform";
import type { ShortcutStatus } from "../../lib/bindings/ShortcutStatus";
import { errorMessages } from "../../lib/messages";
import { captureAccelerator, formatAccelerator } from "../../lib/shortcut";

const DEFAULT_SHORTCUT = "Shift+Super+KeyU";

type ShortcutFieldProps = {
  status: ShortcutStatus;
  platform: Platform;
  pending: boolean;
  onChange: (accelerator: string | null) => Promise<boolean>;
};

function statusText(status: ShortcutStatus, platform: Platform): string {
  switch (status.state) {
    case "registered":
      return "Opens the switcher from anywhere.";
    case "disabled":
      return "Turned off. Open the switcher from the tray.";
    case "conflict":
      return errorMessages.shortcut_conflict;
    case "invalid":
      return errorMessages.shortcut_invalid;
    case "unsupported":
      return platform === "linux"
        ? "Global shortcuts aren't available on this Wayland desktop. Add a custom shortcut in your desktop settings that runs “fluent-account-switcher --toggle”."
        : errorMessages.shortcut_unsupported;
  }
}

export function ShortcutField({ status, platform, pending, onChange }: ShortcutFieldProps) {
  const [capturing, setCapturing] = useState(false);
  const [hint, setHint] = useState<string | null>(null);

  useEffect(() => {
    if (!capturing) return;
    void api.setShortcutCapture(true);
    return () => {
      void api.setShortcutCapture(false);
    };
  }, [capturing]);

  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (!capturing) return;
    event.preventDefault();
    event.stopPropagation();
    const result = captureAccelerator(event);
    if (result.kind === "cancel") {
      setCapturing(false);
      setHint(null);
    } else if (result.kind === "invalid") {
      setHint(errorMessages.shortcut_invalid);
    } else if (result.kind === "accelerator") {
      setCapturing(false);
      setHint(null);
      void onChange(result.value);
    }
  };

  const current = status.accelerator ? formatAccelerator(status.accelerator, platform) : "None";
  const warn =
    status.state === "conflict" || status.state === "unsupported" || status.state === "invalid";

  return (
    <div className="flex flex-col gap-2 py-2">
      <div className="flex items-center justify-between gap-3">
        <span id="shortcut-label" className="text-[13.5px]">
          Shortcut
        </span>
        <button
          type="button"
          aria-labelledby="shortcut-label shortcut-value"
          aria-describedby="shortcut-status"
          aria-pressed={capturing}
          disabled={pending}
          onKeyDown={onKeyDown}
          onBlur={() => {
            setCapturing(false);
          }}
          onClick={() => {
            setCapturing((value) => !value);
            setHint(null);
          }}
          className={`min-w-36 rounded-md border px-2.5 py-1 text-[12.5px] transition-[color,background-color,border-color] duration-100 ${
            capturing ? "border-focus bg-raised" : "border-border bg-control hover:bg-hover"
          }`}
        >
          <span id="shortcut-value">{capturing ? "Press keys… (Esc to cancel)" : current}</span>
        </button>
      </div>
      <p
        id="shortcut-status"
        role={warn ? "alert" : undefined}
        className={`text-[12px] ${warn ? "text-warning" : "text-secondary"}`}
      >
        {hint ?? statusText(status, platform)}
      </p>
      <div className="flex gap-2">
        <button
          type="button"
          disabled={pending || status.accelerator === DEFAULT_SHORTCUT}
          onClick={() => void onChange(DEFAULT_SHORTCUT)}
          className="rounded-md bg-control px-2.5 py-1 text-[12.5px] hover:bg-hover disabled:opacity-40"
        >
          Reset to default
        </button>
        <button
          type="button"
          disabled={pending || status.state === "disabled"}
          onClick={() => void onChange(null)}
          className="rounded-md bg-control px-2.5 py-1 text-[12.5px] hover:bg-hover disabled:opacity-40"
        >
          Turn off
        </button>
      </div>
    </div>
  );
}
