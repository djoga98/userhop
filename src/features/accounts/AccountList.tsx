import { useEffect, useRef, type KeyboardEvent } from "react";

import type { AccountEntry } from "../../lib/bindings/AccountEntry";
import { AccountRow } from "./AccountRow";

type AccountListProps = {
  entries: AccountEntry[];
  compact: boolean;
  pendingKey: string | null;
  locked: boolean;
  hint: string;
  focusKey: string;
  onSelect: (entry: AccountEntry) => void;
};

const NAVIGATION_KEYS = new Set(["ArrowDown", "ArrowUp", "Home", "End"]);

export function AccountList({
  entries,
  compact,
  pendingKey,
  locked,
  hint,
  focusKey,
  onSelect,
}: AccountListProps) {
  const rows = useRef(new Map<string, HTMLButtonElement>());
  const latestEntries = useRef(entries);

  useEffect(() => {
    latestEntries.current = entries;
  });

  useEffect(() => {
    const entries = latestEntries.current;
    const target = entries.find((e) => e.recent) ?? entries[0];
    if (target) rows.current.get(target.key)?.focus();
  }, [focusKey]);

  const onKeyDown = (event: KeyboardEvent<HTMLUListElement>) => {
    if (!NAVIGATION_KEYS.has(event.key) || entries.length === 0) return;
    event.preventDefault();
    const keys = entries.map((e) => e.key);
    const current = keys.findIndex((key) => rows.current.get(key) === document.activeElement);
    const last = keys.length - 1;
    const next =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? last
          : event.key === "ArrowDown"
            ? current < 0
              ? 0
              : Math.min(current + 1, last)
            : current < 0
              ? last
              : Math.max(current - 1, 0);
    const key = keys[next];
    if (key) rows.current.get(key)?.focus();
  };

  return (
    <ul aria-label="Accounts" className="flex flex-col gap-1.5" onKeyDown={onKeyDown}>
      {entries.map((entry) => (
        <li key={entry.key}>
          <AccountRow
            ref={(node) => {
              if (node) rows.current.set(entry.key, node);
              else rows.current.delete(entry.key);
            }}
            entry={entry}
            compact={compact}
            pending={pendingKey === entry.key}
            locked={locked}
            hint={hint}
            onSelect={onSelect}
          />
        </li>
      ))}
    </ul>
  );
}
