import { useEffect, type RefObject } from "react";

import { api } from "./api";

/** Sizes the native window to the panel's natural height so no transparent area captures clicks. */
export function useFlyoutFit(
  chrome: readonly RefObject<HTMLElement | null>[],
  content: RefObject<HTMLElement | null>,
) {
  useEffect(() => {
    if (typeof ResizeObserver === "undefined") return;
    let frame = 0;
    const measure = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const chromeHeight = chrome.reduce((sum, ref) => sum + (ref.current?.offsetHeight ?? 0), 0);
        const contentHeight = content.current?.offsetHeight ?? 0;
        api.fitFlyout(Math.ceil(chromeHeight + contentHeight + 2)).catch((error: unknown) => {
          if (import.meta.env.DEV) console.warn("fit_flyout failed", error);
        });
      });
    };
    const observer = new ResizeObserver(measure);
    for (const ref of [...chrome, content]) {
      if (ref.current) observer.observe(ref.current);
    }
    measure();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [chrome, content]);
}
