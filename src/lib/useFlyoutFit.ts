import { useEffect, type RefObject } from "react";

import { api } from "./api";

type Target = RefObject<HTMLElement | null>;

export type FitTargets = {
  panel: Target;
  scroller: Target;
  content: Target;
  maxHeight: number;
};

/** Untransformed natural height of the panel, so no transparent area captures clicks. */
export function measureNaturalHeight({ panel, scroller, content, maxHeight }: FitTargets): number {
  const chrome = (panel.current?.offsetHeight ?? 0) - (scroller.current?.clientHeight ?? 0);
  return Math.min(chrome + (content.current?.offsetHeight ?? 0) + 1, maxHeight);
}

function reportFailure(error: unknown) {
  if (import.meta.env.DEV) console.warn("flyout sizing failed", error);
}

export function fitFlyout(targets: FitTargets) {
  api.fitFlyout(measureNaturalHeight(targets)).catch(reportFailure);
}

export function flyoutReady(targets: FitTargets) {
  api.flyoutReady(measureNaturalHeight(targets)).catch(reportFailure);
}

/** Follows later content changes; `generation` re-subscribes when the panel is re-created. */
export function useFlyoutFit(targets: FitTargets, generation: number) {
  useEffect(() => {
    if (typeof ResizeObserver === "undefined") return;
    let frame = 0;
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        fitFlyout(targets);
      });
    });
    for (const ref of [targets.panel, targets.content]) {
      if (ref.current) observer.observe(ref.current);
    }
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [targets, generation]);
}
