import type { AccountSnapshot } from "../../lib/bindings/AccountSnapshot";
import type { ErrorCode } from "../../lib/bindings/ErrorCode";

export type SwitcherState =
  | { phase: "hidden"; snapshot: AccountSnapshot | null; requestId: number }
  | { phase: "loading"; snapshot: AccountSnapshot | null; requestId: number }
  | { phase: "ready"; snapshot: AccountSnapshot; requestId: number }
  | { phase: "handoff"; snapshot: AccountSnapshot | null; requestId: number; targetKey: string }
  | {
      phase: "error";
      snapshot: AccountSnapshot | null;
      requestId: number;
      code: ErrorCode;
      source: "load" | "action";
      targetKey: string | null;
    };

export type SwitcherEvent =
  | { type: "open" }
  | { type: "close" }
  | { type: "load-start"; requestId: number }
  | { type: "load-success"; requestId: number; snapshot: AccountSnapshot }
  | { type: "load-failure"; requestId: number; code: ErrorCode }
  | { type: "action-start"; targetKey: string }
  | { type: "action-no-op" }
  | { type: "action-handed-off" }
  | { type: "action-failure"; code: ErrorCode; targetKey: string | null }
  | { type: "dismiss-error" };

export const initialSwitcherState: SwitcherState = {
  phase: "hidden",
  snapshot: null,
  requestId: 0,
};

function ready(state: SwitcherState): SwitcherState {
  return state.snapshot
    ? { phase: "ready", snapshot: state.snapshot, requestId: state.requestId }
    : { phase: "loading", snapshot: null, requestId: state.requestId };
}

export function switcherReducer(state: SwitcherState, event: SwitcherEvent): SwitcherState {
  switch (event.type) {
    case "open":
      return { phase: "loading", snapshot: state.snapshot, requestId: state.requestId };
    case "close":
      return { phase: "hidden", snapshot: state.snapshot, requestId: state.requestId };
    case "load-start":
      if (state.phase === "handoff") return state;
      if (state.phase === "error" && state.source === "action") {
        return { ...state, requestId: event.requestId };
      }
      return { phase: "loading", snapshot: state.snapshot, requestId: event.requestId };
    case "load-success":
      if (event.requestId !== state.requestId || state.phase === "handoff") return state;
      if (state.phase === "error" && state.source === "action") {
        return { ...state, snapshot: event.snapshot };
      }
      return { phase: "ready", snapshot: event.snapshot, requestId: state.requestId };
    case "load-failure":
      if (event.requestId !== state.requestId || state.phase === "handoff") return state;
      return {
        phase: "error",
        snapshot: state.snapshot,
        requestId: state.requestId,
        code: event.code,
        source: "load",
        targetKey: null,
      };
    case "action-start":
      if (state.phase === "handoff") return state;
      return {
        phase: "handoff",
        snapshot: state.snapshot,
        requestId: state.requestId,
        targetKey: event.targetKey,
      };
    case "action-no-op":
      return ready(state);
    case "action-handed-off":
      return { phase: "hidden", snapshot: state.snapshot, requestId: state.requestId };
    case "action-failure":
      return {
        phase: "error",
        snapshot: state.snapshot,
        requestId: state.requestId,
        code: event.code,
        source: "action",
        targetKey: event.targetKey,
      };
    case "dismiss-error":
      return state.phase === "error" ? ready(state) : state;
  }
}
