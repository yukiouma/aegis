import type { ReactNode } from "react";
import { emit } from "@tauri-apps/api/event";
import { AegisThemeProvider, type ThemeMode } from "@aegis/ui/theme";
import { AegisI18nProvider, type Locale } from "@aegis/ui/i18n";

import {
  useHydrateSettingsFromStore,
  useListenForSettingsChanges,
  persistSettings,
} from "../../settings";

/**
 * Glue component mounted once per window inside both provider
 * wrappers. Hydrates theme + locale from the on-disk settings store
 * and subscribes to live changes broadcast by other windows.
 */
export function SettingsSyncBridge({ children }: { children: ReactNode }) {
  useHydrateSettingsFromStore();
  useListenForSettingsChanges();
  return <>{children}</>;
}

/**
 * Wraps AegisThemeProvider so every setMode call in any window is
 * persisted to `settings.bin` AND broadcast to other windows as an
 * `aegis:settings-changed` event. The provider's existing
 * onModeChange callback fires after setMode, so local state has
 * already updated by the time we persist + emit.
 */
export function PersistentThemeProvider({ children }: { children: ReactNode }) {
  const handleChange = async (mode: ThemeMode) => {
    await persistSettings({ theme: mode });
    await emit("aegis:settings-changed", { theme: mode });
  };
  return (
    <AegisThemeProvider onModeChange={handleChange}>
      {children}
    </AegisThemeProvider>
  );
}

/**
 * Wraps AegisI18nProvider with the same persist+broadcast pattern.
 * The default locale falls through to AegisI18nProvider's default
 * ("en") when not supplied.
 */
export function PersistentI18nProvider({
  children,
  defaultLocale,
}: {
  children: ReactNode;
  defaultLocale?: Locale;
}) {
  const handleChange = async (locale: Locale) => {
    await persistSettings({ locale });
    await emit("aegis:settings-changed", { locale });
  };
  return (
    <AegisI18nProvider
      onLocaleChange={handleChange}
      defaultLocale={defaultLocale}
    >
      {children}
    </AegisI18nProvider>
  );
}
