import { en, zhCN } from './locales';
import type { Locale, TranslationKey } from './types';

type TranslationCatalog = Readonly<Record<TranslationKey, string>>;
type TranslationVariables = Record<string, string | number>;

const catalogs = {
  en,
  'zh-CN': zhCN,
} satisfies Record<Locale, TranslationCatalog>;

export function getCatalog(locale: Locale): TranslationCatalog {
  return catalogs[locale];
}

export function resolveMessage(
  catalog: Partial<TranslationCatalog>,
  key: TranslationKey,
): string {
  const englishCatalog: Partial<TranslationCatalog> = en;
  return catalog[key] ?? englishCatalog[key] ?? key;
}

function interpolate(
  message: string,
  variables?: TranslationVariables,
): string {
  if (!variables) return message;

  return message.replace(/\{([A-Za-z0-9_]+)\}/g, (placeholder, name: string) => {
    if (!Object.prototype.hasOwnProperty.call(variables, name)) {
      return placeholder;
    }
    return String(variables[name]);
  });
}

export function translate(
  locale: Locale,
  key: TranslationKey,
  variables?: TranslationVariables,
): string {
  return interpolate(resolveMessage(getCatalog(locale), key), variables);
}