import { emit } from "@tauri-apps/api/event";
import { useEffect, useSyncExternalStore } from "react";

import { App } from "../app/App";
import { heightStore } from "./heightStore";
import type { Scene } from "./scene";

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
      className={`preview-stage preview-${scene.theme} ${scene.center ? "preview-center" : ""}`}
      style={{ width: scene.width, height: scene.height }}
    >
      <div className="preview-glow preview-glow-a" />
      <div className="preview-glow preview-glow-b" />
      <div className="preview-glow preview-glow-c" />
      {scene.hint && (
        <div className="preview-hint" aria-hidden="true">
          <p className="preview-hint-title">Switch accounts in a keystroke</p>
          <p className="preview-keys">
            <kbd>Win</kbd>
            <span>+</span>
            <kbd>Shift</kbd>
            <span>+</span>
            <kbd>U</kbd>
          </p>
          <p className="preview-hint-sub">Your OS signs you in. The app never sees a password.</p>
        </div>
      )}
      <div className="preview-window" style={{ height }}>
        <App />
      </div>
    </div>
  );
}
