import { useEffect, type RefObject } from "react";

import { api } from "./api";

type Target = RefObject<HTMLElement | null>;

const layoutHeight = (ref: Target) => ref.current?.offsetHeight ?? 0;

/** Sizes the native window to the panel's untransformed natural height so no transparent area captures clicks. */
export function useFlyoutFit(panel: Target, scroller: Target, content: Target) {
  useEffect(() => {
    if (typeof ResizeObserver === "undefined") return;
    let frame = 0;
    const measure = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const chrome = layoutHeight(panel) - (scroller.current?.clientHeight ?? 0);
        const natural = chrome + layoutHeight(content) + 1;
        api.fitFlyout(natural).catch((error: unknown) => {
          if (import.meta.env.DEV) console.warn("fit_flyout failed", error);
        });
      });
    };
    const observer = new ResizeObserver(measure);
    for (const ref of [panel, content]) {
      if (ref.current) observer.observe(ref.current);
    }
    measure();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [panel, scroller, content]);
}
