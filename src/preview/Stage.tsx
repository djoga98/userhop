import { emit } from "@tauri-apps/api/event";
import { useEffect, useSyncExternalStore } from "react";

import { App } from "../app/App";
import { heightStore } from "./heightStore";
import type { Scene } from "./scene";
import { Taskbar } from "./Taskbar";

export function Stage({ scene }: { scene: Scene }) {
  const height = useSyncExternalStore(heightStore.subscribe, heightStore.get);
  useEffect(() => {
    const timer = setTimeout(() => {
      if (scene.view === "settings") {
        void emit("flyout://opened", { view: "settings" });
        return;
      }
      const target = document.querySelectorAll<HTMLButtonElement>("[data-row]")[scene.focus];
      if (!target) return;
      target.focus();
      if (scene.notice) target.click();
    }, 400);
    return () => {
      clearTimeout(timer);
    };
  }, [scene]);

  return (
    <div
      className={`desktop preview-stage ${scene.center ? "preview-center" : ""}`}
      data-theme={scene.theme}
      style={{ width: scene.width, height: scene.height }}
    >
      <div
        className={scene.center ? "preview-window" : "desktop-flyout preview-window"}
        style={{ height }}
      >
        <App />
      </div>
      {!scene.center && <Taskbar active />}
    </div>
  );
}
