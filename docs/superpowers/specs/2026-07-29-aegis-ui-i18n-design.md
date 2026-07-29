# @aegis/ui i18n Module Design

**Date:** 2026-07-29
**Status:** Approved for specification

## 1. Goal

Add a dependency-free internationalization module to `@aegis/ui`. The first version supports English (`en`) and Simplified Chinese (`zh-CN`), package-owned translation catalogs, locale switching, persistence, typed translation keys, and simple variable interpolation.

The module follows the provider-and-hook architecture already established by the package's theme module. This change creates the reusable module only; integrating it into the desktop application or migrating existing component strings is outside scope.

## 2. Constraints

- Support exactly `en` and `zh-CN` initially.
- Use the BCP 47 locale identifier `zh-CN` in the public API; the corresponding source file may be named `zhCN.ts`.
- Add no runtime dependencies.
- Keep all translation messages owned by `@aegis/ui`.
- Do not detect the browser or operating-system locale.
- Prefer a saved locale, then an explicit provider default, then English.
- Keep raw catalogs, the registry, the React context, and interpolation helpers internal.
- Export the supported public API from both `@aegis/ui` and `@aegis/ui/i18n`.
- Follow the package's existing TypeScript, Vitest, React Testing Library, and local-storage conventions.

## 3. Architecture

The module will live under `lib/packages/ui/src/i18n`:

```text
src/i18n/
├── locales/
│   ├── en.ts
│   ├── zhCN.ts
│   └── index.ts
├── AegisI18nProvider.tsx
├── AegisI18nProvider.test.tsx
├── registry.ts
├── registry.test.ts
├── types.ts
├── useI18n.ts
└── index.ts
```

Responsibilities are separated as follows:

- `locales/en.ts` defines the canonical English catalog and therefore the valid translation-key set.
- `locales/zhCN.ts` defines the Simplified Chinese catalog and is statically checked against the English key set.
- `locales/index.ts` is an internal barrel used by the registry.
- `registry.ts` maps each supported locale to its catalog and performs defensive fallback lookup.
- `types.ts` defines the public locale, translation-key, provider-prop, and hook-result types.
- `AegisI18nProvider.tsx` owns locale state, initial locale resolution, persistence, callbacks, and the internal context.
- `useI18n.ts` exposes the context through a guarded hook.
- `index.ts` is the focused public barrel for `@aegis/ui/i18n`.

The package root barrel will re-export the same public API, and `package.json` will add:

```json
"./i18n": "./src/i18n/index.ts"
```

## 4. Public API

```ts
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

export function AegisI18nProvider(
  props: AegisI18nProviderProps,
): ReactElement;

export function useI18n(): I18nContextValue;
```

Consumers may import from either public entry point:

```ts
import {
  AegisI18nProvider,
  useI18n,
  type Locale,
  type TranslationKey,
} from '@aegis/ui';
```

```ts
import {
  AegisI18nProvider,
  useI18n,
} from '@aegis/ui/i18n';
```

No raw catalog, registry function, interpolation helper, or React context is public.

## 5. Initial Catalogs

Catalogs use flat, namespaced keys. The initial messages establish both locales without speculatively migrating desktop strings:

```ts
export const en = {
  'language.english': 'English',
  'language.simplifiedChinese': 'Simplified Chinese',
  'language.current': 'Language: {name}',
} as const;
```

```ts
export const zhCN = {
  'language.english': '英语',
  'language.simplifiedChinese': '简体中文',
  'language.current': '当前语言：{name}',
} satisfies Record<keyof typeof en, string>;
```

The English catalog is the source of truth for `TranslationKey`. Every additional locale must satisfy the same key set, so missing and misspelled entries fail type checking.

Future package-owned UI messages will be added to both catalogs in the same change that introduces each key. Existing desktop and Sidebar strings remain unchanged in this work.

## 6. Locale Resolution and State Flow

The provider resolves its initial locale lazily when React initializes state:

1. Read `aegis:i18n:locale` from `localStorage` when browser storage is available.
2. Use the stored value if it is exactly `en` or `zh-CN`.
3. Otherwise use `defaultLocale` when supplied.
4. Otherwise use `en`.

Browser locale detection is deliberately excluded. Changing the `defaultLocale` prop after mount does not replace the active locale; it is an initialization option.

`setLocale` updates provider state and has stable identity across renders. After the initial locale is resolved and after every locale change, an effect:

1. Writes the active locale to `aegis:i18n:locale` when storage is available.
2. Calls `onLocaleChange`, when supplied, with the active locale.

The provider memoizes its context value. The translation function changes only when the active locale changes.

## 7. Translation and Interpolation

`t(key)` looks up the key in the active catalog. Although catalog parity is enforced at compile time, lookup remains defensive for JavaScript consumers and unexpected runtime data:

1. Return the active-locale message when present.
2. Fall back to the English message when the active catalog unexpectedly lacks the key.
3. Return the key itself when neither catalog contains it.

`t(key, variables)` additionally replaces simple `{name}` placeholders with matching string or number values:

```ts
t('language.current', { name: '简体中文' });
```

This first version intentionally does not support plural rules, ICU messages, date formatting, number formatting, nested messages, rich-text values, or asynchronous catalogs.

Interpolation behavior is defensive:

- Values are converted to strings.
- Extra variables are ignored.
- A placeholder with no matching variable remains unchanged so incomplete translations are visible and diagnosable.
- Interpolation never evaluates message text as code or HTML.

## 8. Error Handling

Calling `useI18n` outside the provider throws this explicit error:

```text
useI18n must be used inside <AegisI18nProvider>
```

All storage access is guarded for non-browser environments and wrapped in `try/catch`. A read failure behaves like a missing stored value. A write failure does not prevent rendering, locale switching, translation, or `onLocaleChange` notification.

Invalid stored locale values are ignored and replaced by the selected default when the provider effect next writes the active locale.

Translation lookup and interpolation do not throw for missing runtime data. They use the fallback behavior defined above.

## 9. Testing

Tests are colocated with the implementation and use the package's existing Vitest and React Testing Library setup.

Provider and hook tests will verify:

- English is used when no saved or explicit default locale exists.
- `defaultLocale="zh-CN"` is honored when no valid saved locale exists.
- A valid saved locale takes precedence over `defaultLocale`.
- An invalid saved locale falls back safely.
- `setLocale` changes the active locale and translated output.
- Locale changes are persisted under `aegis:i18n:locale`.
- `onLocaleChange` receives the initially resolved locale and later changes.
- `setLocale` has stable identity across renders.
- `t` returns the expected English and Simplified Chinese messages.
- Simple variable interpolation replaces string and number values.
- Missing variables leave their placeholders unchanged.
- Storage read and write failures do not break rendering or locale switching.
- `useI18n` throws the documented error outside its provider.

Registry and public-surface tests will verify:

- Both supported locales resolve to their expected catalogs.
- Both catalogs contain the same keys.
- Defensive lookup falls back to English and then the key.
- `AegisI18nProvider` and `useI18n` are available from the root barrel.
- The focused `@aegis/ui/i18n` entry point exposes only the intended runtime values and types.

Tests that use storage will install an in-memory `localStorage` shim, matching the existing theme-provider tests.

## 10. Scope Boundaries

This change includes:

- The dependency-free i18n provider, hook, types, catalogs, registry, and tests.
- English and Simplified Chinese catalogs.
- Typed translation keys and simple variable interpolation.
- Locale persistence and change notification.
- Root and focused package exports.

This change excludes:

- Mounting the provider in the desktop application.
- Migrating desktop or existing `@aegis/ui` component strings.
- Browser or operating-system locale detection.
- Runtime catalog extension or consumer-provided messages.
- Pluralization, ICU MessageFormat, dates, numbers, and rich text.
- Lazy loading, remote translation services, and server-side translation.
- Changes to Rust or Tauri code.

These exclusions keep the first version focused while preserving a clear path to add package-owned keys or adopt a richer translation engine if product requirements later justify it.
