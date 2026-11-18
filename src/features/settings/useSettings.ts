import { useCallback, useEffect, useState } from "react";

import { api, toErrorCode } from "../../lib/api";
import type { ErrorCode } from "../../lib/bindings/ErrorCode";
import type { Preferences } from "../../lib/bindings/Preferences";
import type { SettingsView } from "../../lib/bindings/SettingsView";

export function useSettings() {
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [pending, setPending] = useState(true);

  const apply = useCallback(async (operation: () => Promise<SettingsView>) => {
    setPending(true);
    setError(null);
    try {
      setSettings(await operation());
      return true;
    } catch (failure) {
      setError(toErrorCode(failure));
      return false;
    } finally {
      setPending(false);
    }
  }, []);

  const reload = useCallback(() => apply(() => api.getSettings()), [apply]);

  useEffect(() => {
    let active = true;
    api
      .getSettings()
      .then((view) => {
        if (active) setSettings(view);
      })
      .catch((failure: unknown) => {
        if (active) setError(toErrorCode(failure));
      })
      .finally(() => {
        if (active) setPending(false);
      });
    return () => {
      active = false;
    };
  }, []);

  const savePreferences = useCallback(
    (preferences: Preferences) => apply(() => api.saveSettings(preferences)),
    [apply],
  );
  const setShortcut = useCallback(
    (accelerator: string | null) => apply(() => api.setShortcut(accelerator)),
    [apply],
  );
  const setLaunchAtSignIn = useCallback(
    (enabled: boolean) => apply(() => api.setLaunchAtSignIn(enabled)),
    [apply],
  );

  return { settings, error, pending, reload, savePreferences, setShortcut, setLaunchAtSignIn };
}

export type SettingsController = ReturnType<typeof useSettings>;
