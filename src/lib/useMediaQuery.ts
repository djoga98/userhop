import { useCallback, useSyncExternalStore } from "react";

function query(media: string): MediaQueryList | null {
  return typeof window.matchMedia === "function" ? window.matchMedia(media) : null;
}

export function useMediaQuery(media: string): boolean {
  const subscribe = useCallback(
    (notify: () => void) => {
      const list = query(media);
      list?.addEventListener("change", notify);
      return () => list?.removeEventListener("change", notify);
    },
    [media],
  );
  return useSyncExternalStore(subscribe, () => query(media)?.matches ?? false);
}
