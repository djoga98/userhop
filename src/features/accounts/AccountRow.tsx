import { ChevronRight, LoaderCircle } from "lucide-react";
import type { Ref } from "react";

import { Avatar } from "../../components/Avatar";
import type { AccountEntry } from "../../lib/bindings/AccountEntry";
import { actionLabel, statusLabel } from "../../lib/messages";

type AccountRowProps = {
  entry: AccountEntry;
  compact: boolean;
  pending: boolean;
  locked: boolean;
  hint: string;
  onSelect: (entry: AccountEntry) => void;
  ref?: Ref<HTMLButtonElement>;
};

export function AccountRow({
  entry,
  compact,
  pending,
  locked,
  hint,
  onSelect,
  ref,
}: AccountRowProps) {
  const status = statusLabel(entry);
  const action = actionLabel(entry);
  const unavailable = entry.availability.mode === "unavailable";
  const statusId = `status-${entry.key}`;
  const hintId = `hint-${entry.key}`;
  const nameLabel =
    entry.userName && entry.userName !== entry.displayName
      ? `${entry.displayName} (${entry.userName})`
      : entry.displayName;

  return (
    <button
      ref={ref}
      type="button"
      data-row=""
      aria-label={nameLabel}
      aria-describedby={action ? `${statusId} ${hintId}` : statusId}
      aria-disabled={unavailable || locked || undefined}
      aria-busy={pending || undefined}
      aria-current={entry.presence === "current" ? "true" : undefined}
      title={nameLabel}
      onClick={() => {
        if (!locked) onSelect(entry);
      }}
      className={`row-button group flex w-full items-center gap-3 rounded-row px-3 text-left transition-[color,background-color,border-color] duration-100 hover:bg-hover active:bg-pressed ${
        compact ? "min-h-12 py-1.5" : "min-h-[60px] py-2"
      } ${unavailable ? "text-secondary" : ""}`}
    >
      <Avatar
        id={entry.accountId}
        displayName={entry.displayName}
        userName={entry.userName}
        compact={compact}
      />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[13.5px] font-semibold">{entry.displayName}</span>
        <span id={statusId} className="block truncate text-[12px] text-secondary">
          {status}
        </span>
        {action && (
          <span id={hintId} className="sr-only">
            {`${action}. ${hint}`}
          </span>
        )}
      </span>
      {pending ? (
        <LoaderCircle aria-hidden="true" size={16} className="spin shrink-0 text-secondary" />
      ) : entry.presence === "current" ? (
        <span className="shrink-0 text-[12px] text-secondary">Current</span>
      ) : action ? (
        <span className="flex shrink-0 items-center gap-1 text-[12px] text-secondary">
          <span className="opacity-0 transition-opacity duration-100 group-hover:opacity-100 group-focus-visible:opacity-100">
            {action}
          </span>
          <ChevronRight aria-hidden="true" size={16} />
        </span>
      ) : null}
    </button>
  );
}
