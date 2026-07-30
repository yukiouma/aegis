import type { ReactNode } from 'react';
import type { en } from './locales/en';

export type Locale = 'en' | 'zh-CN';
export type TranslationKey = keyof typeof en;

export interface AegisI18nProviderProps {
  children: ReactNode;
  defaultLocale?: Locale;
  onLocaleChange?: (locale: Locale) => void;
}

export interface I18nContextValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (
    key: TranslationKey,
    variables?: Record<string, string | number>,
  ) => string;
}