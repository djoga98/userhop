import { mockIPC } from "@tauri-apps/api/mocks";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "../styles/index.css";
import "../../site/desktop.css";
import "./preview.css";
import "./video.css";
import { heightStore } from "./heightStore";
import { readScene, sampleCapabilitiesFor, sampleSettings, sampleSnapshotFor } from "./scene";
import { Stage } from "./Stage";
import { VideoStage } from "./VideoStage";

const scene = readScene(window.location.search);

function commandError(code: string) {
  return Object.assign(new Error(code), { code });
}

mockIPC(
  (command, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    switch (command) {
      case "get_accounts":
        return sampleSnapshotFor(scene);
      case "get_capabilities":
        return sampleCapabilitiesFor(scene);
      case "get_settings":
        return sampleSettings(scene);
      case "save_settings":
        return {
          ...sampleSettings(scene),
          preferences: args.preferences as ReturnType<typeof sampleSettings>["preferences"],
        };
      case "request_switch":
        throw commandError(scene.notice ? "remote_session" : "busy");
      case "fit_flyout":
        heightStore.set(Number(args.height));
        return null;
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      {new URLSearchParams(window.location.search).has("video") ? (
        <VideoStage />
      ) : (
        <Stage scene={scene} />
      )}
    </StrictMode>,
  );
}
