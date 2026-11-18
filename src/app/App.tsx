import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import { AccountsView } from "../features/accounts/AccountsView";
import { useSwitcher } from "../features/accounts/useSwitcher";
import { SettingsView } from "../features/settings/SettingsView";
import { useSettings } from "../features/settings/useSettings";
import { api, events } from "../lib/api";
import type { FlyoutView } from "../lib/bindings/FlyoutView";
import type { PlatformCapabilities } from "../lib/bindings/PlatformCapabilities";
import { fitFlyout, flyoutReady, useFlyoutFit } from "../lib/useFlyoutFit";
import { useMediaQuery } from "../lib/useMediaQuery";
import { Footer } from "./Footer";
import { Header } from "./Header";

const REFRESH_DEBOUNCE_MS = 250;
const MAX_HEIGHT: Record<FlyoutView, number> = { list: 520, settings: 700 };
const FOCUSABLE = "button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex='-1'])";
const CLOSE_ANIMATION_MS = 120;

export function App() {
  const switcher = useSwitcher();
  const settingsController = useSettings();
  const [capabilities, setCapabilities] = useState<PlatformCapabilities | null>(null);
  const [view, setView] = useState<FlyoutView>("list");
  const [openCount, setOpenCount] = useState(1);
  const [listFocusToken, setListFocusToken] = useState(1);
  const [closing, setClosing] = useState(false);
  const prefersDark = useMediaQuery("(prefers-color-scheme: dark)");
  const prefersReducedMotion = useMediaQuery("(prefers-reduced-motion: reduce)");
  const panelRef = useRef<HTMLDivElement>(null);
  const scrollerRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const { open, close, refresh } = switcher;
  const reloadSettings = settingsController.reload;
  const visible = switcher.state.phase !== "hidden";
  const visibleRef = useRef(visible);

  useEffect(() => {
    visibleRef.current = visible;
  }, [visible]);

  const fitTargets = useMemo(
    () => ({
      panel: panelRef,
      scroller: scrollerRef,
      content: contentRef,
      maxHeight: MAX_HEIGHT[view],
    }),
    [view],
  );
  useFlyoutFit(fitTargets, openCount);
  const announcedOpen = useRef(0);

  useLayoutEffect(() => {
    if (announcedOpen.current !== openCount) {
      announcedOpen.current = openCount;
      flyoutReady(fitTargets);
    } else {
      fitFlyout(fitTargets);
    }
  }, [fitTargets, openCount]);

  const loadCapabilities = useCallback(async () => {
    try {
      setCapabilities(await api.getCapabilities());
    } catch (error) {
      if (import.meta.env.DEV) console.warn("get_capabilities failed", error);
      setCapabilities(null);
    }
  }, []);

  const onOpened = useCallback(
    (requested: FlyoutView) => {
      setView(requested);
      setClosing(false);
      setOpenCount((count) => count + 1);
      open();
      void loadCapabilities();
      void reloadSettings();
    },
    [open, loadCapabilities, reloadSettings],
  );

  useEffect(() => {
    let active = true;
    api
      .getCapabilities()
      .then((result) => {
        if (active) setCapabilities(result);
      })
      .catch((error: unknown) => {
        if (import.meta.env.DEV) console.warn("get_capabilities failed", error);
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const subscriptions = [
      events.onFlyoutOpened(({ view: requested }) => {
        onOpened(requested);
      }),
      events.onFlyoutHidden(() => {
        close();
      }),
      events.onSessionsChanged(() => {
        clearTimeout(timer);
        timer = setTimeout(() => {
          if (visibleRef.current) void refresh();
        }, REFRESH_DEBOUNCE_MS);
      }),
    ];
    return () => {
      clearTimeout(timer);
      for (const subscription of subscriptions) {
        void subscription.then((unlisten) => {
          unlisten();
        });
      }
    };
  }, [onOpened, close, refresh]);

  const preferences = settingsController.settings?.preferences;
  const reducedMotion = prefersReducedMotion || preferences?.reduceMotion === true;
  const nativeBackdrop = capabilities !== null && capabilities.backdrop !== "none";
  const animated = !reducedMotion && !nativeBackdrop;
  const theme = preferences?.theme ?? "system";
  const resolvedTheme = theme === "system" ? (prefersDark ? "dark" : "light") : theme;

  const requestClose = useCallback(() => {
    if (closing) return;
    setClosing(true);
    setTimeout(
      () => {
        void api.hideFlyout();
      },
      animated ? CLOSE_ANIMATION_MS : 0,
    );
  }, [closing, animated]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Tab" && !event.defaultPrevented) {
        const focusable = Array.from(
          panelRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? [],
        );
        const first = focusable[0];
        const last = focusable.at(-1);
        const active = document.activeElement;
        const outside = !panelRef.current?.contains(active);
        if (first && last && (outside || active === (event.shiftKey ? first : last))) {
          event.preventDefault();
          (event.shiftKey ? last : first).focus();
        }
        return;
      }
      if (event.key !== "Escape" || event.defaultPrevented) return;
      event.preventDefault();
      if (view === "settings") {
        setView("list");
        setListFocusToken((token) => token + 1);
      } else requestClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [view, requestClose]);

  const platform = capabilities?.platform ?? "other";
  const busy = switcher.state.phase === "handoff";

  return (
    <div
      className="h-full"
      data-resolved-theme={resolvedTheme}
      data-platform={capabilities?.platform ?? "other"}
      data-backdrop={capabilities?.backdrop ?? "none"}
      data-reduced-motion={reducedMotion ? "true" : "false"}
    >
      <div
        key={openCount}
        ref={panelRef}
        role="dialog"
        aria-labelledby="flyout-title"
        className={`flyout-panel flex h-full flex-col overflow-hidden text-text ${
          animated ? (closing ? "flyout-leave" : "flyout-enter") : ""
        }`}
      >
        <Header
          view={view}
          onBack={() => {
            setView("list");
            setListFocusToken((token) => token + 1);
          }}
          onSettings={() => {
            setView("settings");
          }}
          onClose={requestClose}
        />
        <div
          ref={scrollerRef}
          className="flyout-scroll min-h-0 flex-1 overflow-y-auto border-t border-separator"
        >
          <div ref={contentRef} className={view === "list" ? "p-2" : "py-1"}>
            {view === "list" ? (
              <AccountsView
                state={switcher.state}
                capabilities={capabilities}
                compact={preferences?.compactRows === true}
                focusKey={`${openCount}:${listFocusToken}`}
                onSelect={switcher.select}
                onRefresh={() => void refresh()}
                onOpenSignIn={switcher.openSignIn}
                onLock={switcher.lock}
                onDismissError={switcher.dismissError}
              />
            ) : (
              <SettingsView controller={settingsController} platform={platform} />
            )}
          </div>
        </div>
        <Footer
          shortcut={settingsController.settings?.shortcut ?? null}
          platform={platform}
          lockAvailable={capabilities?.lockScreen.status === "available"}
          busy={busy}
          onLock={switcher.lock}
        />
      </div>
    </div>
  );
}
