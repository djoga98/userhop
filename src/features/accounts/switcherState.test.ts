import { describe, expect, it } from "vitest";

import { snapshot } from "../../test/fixtures";
import { initialSwitcherState, switcherReducer, type SwitcherState } from "./switcherState";

const loaded: SwitcherState = { phase: "ready", snapshot: snapshot(), requestId: 1 };

describe("switcherReducer", () => {
  it("drops list responses that belong to an older request", () => {
    let state = switcherReducer(initialSwitcherState, { type: "open" });
    state = switcherReducer(state, { type: "load-start", requestId: 1 });
    state = switcherReducer(state, { type: "load-start", requestId: 2 });
    state = switcherReducer(state, { type: "load-success", requestId: 1, snapshot: snapshot([]) });
    expect(state.phase).toBe("loading");
    state = switcherReducer(state, { type: "load-success", requestId: 2, snapshot: snapshot() });
    expect(state).toMatchObject({ phase: "ready", requestId: 2 });
  });

  it("ignores refreshes and second actions while a handoff is in flight", () => {
    let state = switcherReducer(loaded, { type: "action-start", targetKey: "account:t:2" });
    state = switcherReducer(state, { type: "load-start", requestId: 5 });
    state = switcherReducer(state, { type: "action-start", targetKey: "account:t:3" });
    expect(state).toMatchObject({ phase: "handoff", targetKey: "account:t:2" });
  });

  it("hands off without ever claiming a completed switch", () => {
    let state = switcherReducer(loaded, { type: "action-start", targetKey: "account:t:2" });
    state = switcherReducer(state, { type: "action-handed-off" });
    expect(state.phase).toBe("hidden");
  });

  it("keeps the list visible on action failure and returns to ready on dismiss", () => {
    let state = switcherReducer(loaded, { type: "action-start", targetKey: "account:t:2" });
    state = switcherReducer(state, {
      type: "action-failure",
      code: "permission_denied",
      targetKey: "account:t:2",
    });
    expect(state).toMatchObject({ phase: "error", source: "action", code: "permission_denied" });
    expect(state.snapshot).not.toBeNull();
    state = switcherReducer(state, { type: "load-start", requestId: 9 });
    state = switcherReducer(state, { type: "load-success", requestId: 9, snapshot: snapshot([]) });
    expect(state).toMatchObject({ phase: "error", source: "action" });
    state = switcherReducer(state, { type: "dismiss-error" });
    expect(state.phase).toBe("ready");
  });

  it("reports load failures for the latest request only", () => {
    let state = switcherReducer(initialSwitcherState, { type: "load-start", requestId: 3 });
    state = switcherReducer(state, { type: "load-failure", requestId: 2, code: "os_api_failure" });
    expect(state.phase).toBe("loading");
    state = switcherReducer(state, { type: "load-failure", requestId: 3, code: "os_api_failure" });
    expect(state).toMatchObject({ phase: "error", source: "load" });
  });
});
