import {
  createContext,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react';
import { translate } from './registry';
import type {
  AegisI18nProviderProps,
  I18nContextValue,
  Locale,
} from './types';

const STORAGE_KEY = 'aegis:i18n:locale';

function isLocale(value: string | null): value is Locale {
  return value === 'en' || value === 'zh-CN';
}

function readInitialLocale(defaultLocale: Locale): Locale {
  if (typeof window === 'undefined') return defaultLocale;

  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (isLocale(stored)) return stored;
  } catch {
    // localStorage may throw in private modes or sandboxed contexts.
  }

  return defaultLocale;
}

export const AegisI18nContext = createContext<I18nContextValue | null>(null);

export function AegisI18nProvider({
  children,
  defaultLocale = 'en',
  onLocaleChange,
}: AegisI18nProviderProps) {
  const [locale, setLocaleState] = useState<Locale>(() =>
    readInitialLocale(defaultLocale),
  );

  const setLocale = useCallback((nextLocale: Locale) => {
    setLocaleState(nextLocale);
  }, []);

  const t = useCallback<I18nContextValue['t']>(
    (key, variables) => translate(locale, key, variables),
    [locale],
  );

  const value = useMemo<I18nContextValue>(
    () => ({ locale, setLocale, t }),
    [locale, setLocale, t],
  );

  useEffect(() => {
    if (typeof window !== 'undefined') {
      try {
        window.localStorage.setItem(STORAGE_KEY, locale);
      } catch {
        // Ignore storage failures; in-memory locale state remains usable.
      }
    }

    onLocaleChange?.(locale);
  }, [locale, onLocaleChange]);

  return (
    <AegisI18nContext.Provider value={value}>
      {children}
    </AegisI18nContext.Provider>
  );
}