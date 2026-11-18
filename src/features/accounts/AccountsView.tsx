import { LoaderCircle, LogIn } from "lucide-react";

import { Notice, NoticeAction } from "../../components/Notice";
import type { AccountEntry } from "../../lib/bindings/AccountEntry";
import type { ErrorCode } from "../../lib/bindings/ErrorCode";
import type { PlatformCapabilities } from "../../lib/bindings/PlatformCapabilities";
import { errorMessages, handoffDescription, staleCodes } from "../../lib/messages";
import { AccountList } from "./AccountList";
import type { SwitcherState } from "./switcherState";
import { LOCK_TARGET, SIGN_IN_TARGET } from "./useSwitcher";

type AccountsViewProps = {
  state: SwitcherState;
  capabilities: PlatformCapabilities | null;
  compact: boolean;
  focusKey: string;
  onSelect: (entry: AccountEntry) => void;
  onRefresh: () => void;
  onOpenSignIn: () => void;
  onLock: () => void;
  onDismissError: () => void;
};

function ErrorActions({
  code,
  targetEntry,
  capabilities,
  onRetry,
  onOpenSignIn,
  onLock,
  onDismiss,
}: {
  code: ErrorCode;
  targetEntry: AccountEntry | null;
  capabilities: PlatformCapabilities | null;
  onRetry: (() => void) | null;
  onOpenSignIn: () => void;
  onLock: () => void;
  onDismiss: () => void;
}) {
  const signInAvailable = capabilities?.systemSignIn.status === "available";
  const lockAvailable = capabilities?.lockScreen.status === "available";
  const retryable =
    onRetry !== null &&
    !staleCodes.includes(code) &&
    targetEntry?.availability.mode !== "unavailable" &&
    code !== "secure_handoff_unavailable";
  return (
    <>
      {retryable && (
        <NoticeAction primary onClick={onRetry}>
          Try again
        </NoticeAction>
      )}
      {code === "secure_handoff_unavailable" && lockAvailable && (
        <NoticeAction primary onClick={onLock}>
          Lock screen
        </NoticeAction>
      )}
      {signInAvailable && code !== "secure_handoff_unavailable" && !retryable && (
        <NoticeAction onClick={onOpenSignIn}>Open system sign-in</NoticeAction>
      )}
      <NoticeAction onClick={onDismiss}>Dismiss</NoticeAction>
    </>
  );
}

export function AccountsView({
  state,
  capabilities,
  compact,
  focusKey,
  onSelect,
  onRefresh,
  onOpenSignIn,
  onLock,
  onDismissError,
}: AccountsViewProps) {
  const snapshot = state.snapshot;
  const signInAvailable = capabilities?.systemSignIn.status === "available";
  const hint = handoffDescription(capabilities?.signInSurface ?? "none");
  const busy = state.phase === "handoff";
  const pendingKey = state.phase === "handoff" ? state.targetKey : null;

  if (!snapshot) {
    if (state.phase === "error") {
      return (
        <Notice message={`Accounts couldn't be read. ${errorMessages[state.code]}`}>
          <NoticeAction primary onClick={onRefresh}>
            Try again
          </NoticeAction>
          {signInAvailable && (
            <NoticeAction onClick={onOpenSignIn}>Open system sign-in</NoticeAction>
          )}
        </Notice>
      );
    }
    return (
      <p role="status" className="flex items-center gap-2 px-3 py-4 text-[13px] text-secondary">
        <LoaderCircle aria-hidden="true" size={16} className="spin" />
        Reading accounts…
      </p>
    );
  }

  const others = snapshot.entries.filter((e) => !e.isCurrentUser);
  const targetEntry =
    state.phase === "error"
      ? (snapshot.entries.find((e) => e.key === state.targetKey) ?? null)
      : null;
  const retry = (() => {
    if (state.phase !== "error") return null;
    if (state.source === "load") return onRefresh;
    if (state.targetKey === SIGN_IN_TARGET) return onOpenSignIn;
    if (state.targetKey === LOCK_TARGET) return onLock;
    return targetEntry
      ? () => {
          onSelect(targetEntry);
        }
      : null;
  })();
  const showSignInRow = signInAvailable && (others.length === 0 || !snapshot.directoryComplete);

  return (
    <div className="flex flex-col">
      {state.phase === "error" && (
        <Notice message={errorMessages[state.code]}>
          <ErrorActions
            code={state.code}
            targetEntry={targetEntry}
            capabilities={capabilities}
            onRetry={retry}
            onOpenSignIn={onOpenSignIn}
            onLock={onLock}
            onDismiss={onDismissError}
          />
        </Notice>
      )}
      <AccountList
        entries={snapshot.entries}
        compact={compact}
        pendingKey={pendingKey}
        locked={busy}
        hint={hint}
        focusKey={focusKey}
        onSelect={onSelect}
      />
      {showSignInRow && (
        <button
          type="button"
          aria-describedby="sign-in-row-hint"
          aria-disabled={busy || undefined}
          onClick={() => {
            if (!busy) onOpenSignIn();
          }}
          className={`row-button mt-1.5 flex w-full items-center gap-3 rounded-row px-3 text-left text-[13.5px] transition-colors duration-100 hover:bg-hover active:bg-pressed ${
            compact ? "min-h-12" : "min-h-[52px]"
          }`}
        >
          <span
            aria-hidden="true"
            className={`grid shrink-0 place-items-center rounded-full bg-control ${compact ? "size-8" : "size-10"}`}
          >
            {pendingKey === SIGN_IN_TARGET ? (
              <LoaderCircle size={16} className="spin" />
            ) : (
              <LogIn size={16} />
            )}
          </span>
          <span className="min-w-0 flex-1">
            <span className="block">
              {others.length === 0 ? "Use another account" : "Other accounts"}
            </span>
            <span id="sign-in-row-hint" className="block truncate text-[12px] text-secondary">
              {hint}
            </span>
          </span>
        </button>
      )}
      {!snapshot.directoryComplete && (
        <p className="px-3 pt-2 text-[12px] text-secondary">
          Only accounts with a running session can be listed on this system.
        </p>
      )}
      {!signInAvailable && capabilities?.systemSignIn.status === "unavailable" && (
        <p className="px-3 pt-2 text-[12px] text-secondary">
          {errorMessages[capabilities.systemSignIn.reason]}
        </p>
      )}
    </div>
  );
}
