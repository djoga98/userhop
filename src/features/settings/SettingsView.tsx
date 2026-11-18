import { LoaderCircle } from "lucide-react";
import type { ReactNode } from "react";

import { Notice, NoticeAction } from "../../components/Notice";
import { Toggle } from "../../components/Toggle";
import type { Platform } from "../../lib/bindings/Platform";
import type { ThemePreference } from "../../lib/bindings/ThemePreference";
import { errorMessages } from "../../lib/messages";
import { ShortcutField } from "./ShortcutField";
import type { SettingsController } from "./useSettings";

const THEMES: { value: ThemePreference; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section aria-label={title} className="border-b border-separator px-3 py-2 last:border-b-0">
      <h2 className="pt-1 text-[12px] font-semibold text-secondary">{title}</h2>
      {children}
    </section>
  );
}

type SettingsViewProps = {
  controller: SettingsController;
  platform: Platform;
};

export function SettingsView({ controller, platform }: SettingsViewProps) {
  const { settings, error, pending } = controller;

  if (!settings) {
    return error ? (
      <Notice message={errorMessages[error]}>
        <NoticeAction primary onClick={() => void controller.reload()}>
          Try again
        </NoticeAction>
      </Notice>
    ) : (
      <p role="status" className="flex items-center gap-2 px-3 py-4 text-[13px] text-secondary">
        <LoaderCircle aria-hidden="true" size={16} className="spin" />
        Loading settings…
      </p>
    );
  }

  const { preferences, launchAtSignIn } = settings;
  const save = (patch: Partial<typeof preferences>) =>
    void controller.savePreferences({ ...preferences, ...patch });

  return (
    <div className="view-enter flex flex-col">
      {error && <Notice message={errorMessages[error]} />}
      <Section title="Startup">
        <Toggle
          id="launch-at-sign-in"
          label="Launch at sign-in"
          description={
            launchAtSignIn.capability.status === "available"
              ? "Starts quietly in the tray for this account only."
              : errorMessages[launchAtSignIn.capability.reason]
          }
          checked={launchAtSignIn.enabled}
          disabled={pending || launchAtSignIn.capability.status !== "available"}
          onChange={(enabled) => void controller.setLaunchAtSignIn(enabled)}
        />
      </Section>
      <Section title="Keyboard">
        <ShortcutField
          status={settings.shortcut}
          defaultAccelerator={settings.defaultShortcut}
          platform={platform}
          pending={pending}
          onChange={controller.setShortcut}
        />
      </Section>
      <Section title="Appearance">
        <fieldset className="py-2">
          <legend className="sr-only">Theme</legend>
          <div className="grid grid-cols-3 gap-1 rounded-md bg-control p-0.5">
            {THEMES.map((theme) => (
              <label
                key={theme.value}
                className="relative rounded-[5px] py-1 text-center text-[12.5px] has-checked:bg-raised has-checked:font-semibold has-checked:shadow-sm has-focus-visible:outline-2 has-focus-visible:outline-focus"
              >
                <input
                  type="radio"
                  name="theme"
                  value={theme.value}
                  checked={preferences.theme === theme.value}
                  disabled={pending}
                  onChange={() => {
                    save({ theme: theme.value });
                  }}
                  className="absolute inset-0 cursor-default opacity-0"
                />
                {theme.label}
              </label>
            ))}
          </div>
        </fieldset>
        <Toggle
          id="compact-rows"
          label="Compact rows"
          checked={preferences.compactRows}
          disabled={pending}
          onChange={(compactRows) => {
            save({ compactRows });
          }}
        />
        <Toggle
          id="reduce-motion"
          label="Reduce motion"
          description="Animations already follow your system setting."
          checked={preferences.reduceMotion}
          disabled={pending}
          onChange={(reduceMotion) => {
            save({ reduceMotion });
          }}
        />
      </Section>
      <Section title="About">
        <p className="py-2 text-[12.5px] text-secondary">
          Fluent Account Switcher {settings.version} · MIT License
          <br />
          Sign-in is always handled by your operating system. No passwords, accounts or usage data
          are stored or sent anywhere.
        </p>
      </Section>
    </div>
  );
}
