import { useCallback, useEffect, useReducer, useRef } from "react";

import { api, toErrorCode } from "../../lib/api";
import type { AccountEntry } from "../../lib/bindings/AccountEntry";
import type { SwitchResult } from "../../lib/bindings/SwitchResult";
import { staleCodes } from "../../lib/messages";
import { switcherReducer, type SwitcherState } from "./switcherState";

export const SIGN_IN_TARGET = "system-sign-in";
export const LOCK_TARGET = "lock";

const INITIAL_STATE: SwitcherState = { phase: "loading", snapshot: null, requestId: 1 };

export function useSwitcher() {
  const [state, dispatch] = useReducer(switcherReducer, INITIAL_STATE);
  const requestCounter = useRef(INITIAL_STATE.requestId);
  const inFlight = useRef(false);

  const load = useCallback(async (requestId: number) => {
    try {
      const snapshot = await api.getAccounts();
      dispatch({ type: "load-success", requestId, snapshot });
    } catch (error) {
      dispatch({ type: "load-failure", requestId, code: toErrorCode(error) });
    }
  }, []);

  useEffect(() => {
    void load(INITIAL_STATE.requestId);
  }, [load]);

  const refresh = useCallback(() => {
    requestCounter.current += 1;
    const requestId = requestCounter.current;
    dispatch({ type: "load-start", requestId });
    return load(requestId);
  }, [load]);

  const run = useCallback(
    async (targetKey: string, action: () => Promise<SwitchResult | null>) => {
      if (inFlight.current) return;
      inFlight.current = true;
      dispatch({ type: "action-start", targetKey });
      try {
        const result = await action();
        dispatch(
          result?.kind === "no-op-current-session"
            ? { type: "action-no-op" }
            : { type: "action-handed-off" },
        );
      } catch (error) {
        const code = toErrorCode(error);
        dispatch({ type: "action-failure", code, targetKey });
        if (staleCodes.includes(code)) void refresh();
      } finally {
        inFlight.current = false;
      }
    },
    [refresh],
  );

  const select = useCallback(
    (entry: AccountEntry) => {
      if (entry.availability.mode === "current") return;
      if (entry.availability.mode === "unavailable") {
        dispatch({ type: "action-failure", code: entry.availability.reason, targetKey: entry.key });
        return;
      }
      void run(entry.key, () =>
        api.requestSwitch({ accountId: entry.accountId, sessionId: entry.sessionId }),
      );
    },
    [run],
  );

  const openSignIn = useCallback(() => {
    void run(SIGN_IN_TARGET, () => api.openSystemSignIn());
  }, [run]);

  const lock = useCallback(() => {
    void run(LOCK_TARGET, async () => {
      await api.lockSession();
      return null;
    });
  }, [run]);

  const open = useCallback(() => {
    dispatch({ type: "open" });
    void refresh();
  }, [refresh]);

  const close = useCallback(() => {
    dispatch({ type: "close" });
  }, []);

  const dismissError = useCallback(() => {
    dispatch({ type: "dismiss-error" });
  }, []);

  return { state, refresh, select, openSignIn, lock, open, close, dismissError };
}
